use super::{ActiveOverlay, GameState, text};
use crate::dim::{ActiveDimension, Dimension, DimensionGenerationRequest};
use crate::pld::meta::ReloadMetadataRequest;
use crate::pld::{
    EvictPayloadScopeRequest, FetchPayloadScopeRequest, MountPayloadSourceRequest,
    PayloadFetchingComplete, PayloadScopeId, RefetchPayloadRequest, UnmountPayloadSourceRequest,
    ui_image_node,
};
use crate::utils::{Keyed, NamespacedKey};
use bevy::app::App;
use bevy::asset::io::AssetSourceId;
use bevy::color::palettes::css::{WHITE, YELLOW};
use bevy::ecs::query::{QueryData, ReadOnlyQueryData};
use bevy::ecs::system::ObserverSystem;
use bevy::ecs::system::command::{insert_resource, trigger};
use bevy::log::tracing::Span;
use bevy::log::tracing::span::Entered;
use bevy::prelude::*;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Debug, Event)]
pub enum Load {
    EnterDimension(DimensionEntryContext, NamespacedKey),
    EnterMainMenu(MainMenuEntryContext),
    Reload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DimensionEntryContext {
    EnterWorld,
    GatewayTravel,
    PortalTravel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MainMenuEntryContext {
    Init,
    ExitWorld,
    SaveAndExitWorld,
}

// TODO: Fix
// Warning: Faulty logic ahead

#[derive(Component, Default)]
struct LoadingTask;

trait LoadingTaskComponent: Component + Debug {
    fn task(&self) -> impl Command;
}

#[derive(Component, Debug)]
#[require(LoadingTask)]
struct GameStateTransitionTask(GameState);

impl LoadingTaskComponent for GameStateTransitionTask {
    fn task(&self) -> impl Command {
        insert_resource(NextState::Pending(self.0))
    }
}

#[derive(Component, Debug)]
#[require(LoadingTask)]
struct DimensionGenerationTask(NamespacedKey);

impl LoadingTaskComponent for DimensionGenerationTask {
    fn task(&self) -> impl Command {
        trigger(DimensionGenerationRequest::new(&self.0))
    }
}

#[derive(Component, Debug)]
#[require(LoadingTask)]
struct WorldSavingTask;

impl LoadingTaskComponent for WorldSavingTask {
    fn task(&self) -> impl Command {
        |_world: &mut World| {}
    }
}

#[derive(Component, Debug)]
#[require(LoadingTask)]
struct FetchPayloadScopeTask(PayloadScopeId);

impl LoadingTaskComponent for FetchPayloadScopeTask {
    fn task(&self) -> impl Command {
        trigger(FetchPayloadScopeRequest::new(self.0.clone()))
    }
}

#[derive(Component, Debug)]
#[require(LoadingTask)]
struct MountPayloadSourceTask(AssetSourceId<'static>);

impl LoadingTaskComponent for MountPayloadSourceTask {
    fn task(&self) -> impl Command {
        trigger(MountPayloadSourceRequest::new(self.0.clone()))
    }
}

#[derive(Component, Debug)]
#[require(LoadingTask)]
struct RefetchPayloadTask;

impl LoadingTaskComponent for RefetchPayloadTask {
    fn task(&self) -> impl Command {
        trigger(RefetchPayloadRequest)
    }
}

#[derive(Component, Debug)]
#[require(LoadingTask)]
struct EvictPayloadScopeTask(PayloadScopeId);

impl LoadingTaskComponent for EvictPayloadScopeTask {
    fn task(&self) -> impl Command {
        trigger(EvictPayloadScopeRequest::new(self.0.clone()))
    }
}

#[derive(Component, Debug)]
#[require(LoadingTask)]
struct UnmountPayloadSourceTask(AssetSourceId<'static>);

impl LoadingTaskComponent for UnmountPayloadSourceTask {
    fn task(&self) -> impl Command {
        trigger(UnmountPayloadSourceRequest::new(self.0.clone()))
    }
}

#[derive(Component, Debug)]
#[require(LoadingTask)]
struct ReloadMetadataTask;

impl LoadingTaskComponent for ReloadMetadataTask {
    fn task(&self) -> impl Command {
        trigger(ReloadMetadataRequest)
    }
}

#[derive(Component)]
#[relationship(relationship_target = TaskDependencies)]
struct TaskDependent(Entity);

#[derive(Component, Default)]
#[relationship_target(relationship = TaskDependent)]
struct TaskDependencies(Vec<Entity>);

#[derive(Default, Resource)]
struct LoadingSpan(Option<Span>);

impl LoadingSpan {
    #[inline]
    fn enter(&self) -> Entered<'_> {
        self.0
            .as_ref()
            .expect("The loading span should exist.")
            .enter()
    }
}

#[derive(Debug, Default, Resource)]
struct LoadingOverlay(ActiveOverlay);

// We track dependencies separately because we need immediate mutation, not a deferred command

#[derive(Default, Resource)]
struct PendingTaskCount(usize);

#[derive(Component)]
struct RemainingTaskDependencyCount(usize);

#[derive(Event)]
struct InitializeTasks;

#[derive(Debug, Event)]
struct BeginTask(Entity);

fn begin_loading(
    loading: On<Load>,
    mut loading_span: ResMut<LoadingSpan>,
    mut commands: Commands,
    active_dimension: Option<Single<&Dimension, With<ActiveDimension>>>,
    active_overlay: Res<State<ActiveOverlay>>,
    mut loading_overlay: ResMut<LoadingOverlay>,
    mut next_overlay: ResMut<NextState<ActiveOverlay>>,
    mut settings: ResMut<LoadingScreenSettings>,
) {
    let span = info_span!("loading", load = ?*loading);
    let entered = span.enter();
    info!("Begin loading");
    *loading_overlay = LoadingOverlay(match &*loading {
        Load::EnterDimension(_context, _key) => ActiveOverlay::HeadsUpDisplay,
        Load::EnterMainMenu(_context) => ActiveOverlay::TitleScreen,
        Load::Reload => **active_overlay,
    });
    next_overlay.set(ActiveOverlay::LoadingScreen);
    let loading = &*loading;
    *settings = match loading {
        Load::EnterDimension(context, dimension) => LoadingScreenSettings {
            load_tip: Some(match context {
                DimensionEntryContext::EnterWorld => "Joining".to_string(),
                DimensionEntryContext::GatewayTravel => "Preparing warp".to_string(),
                DimensionEntryContext::PortalTravel => "Traveling to".to_string(),
            }),
            target_tip: Some(dimension.to_string()),
            background: None,
        },
        Load::EnterMainMenu(
            MainMenuEntryContext::ExitWorld | MainMenuEntryContext::SaveAndExitWorld,
        ) => LoadingScreenSettings {
            load_tip: Some("Leaving".to_string()),
            target_tip: None,
            background: None,
        },
        Load::EnterMainMenu(MainMenuEntryContext::Init) | Load::Reload => LoadingScreenSettings {
            load_tip: None,
            target_tip: None,
            background: None,
        },
    };
    match loading {
        Load::EnterDimension(context, dimension_key) => match context {
            DimensionEntryContext::EnterWorld => {
                commands.spawn((
                    DimensionGenerationTask(dimension_key.clone()),
                    related!(
                        TaskDependencies[(
                            ReloadMetadataTask,
                            related!(
                                TaskDependencies[(
                                    FetchPayloadScopeTask(PayloadScopeId::Dimension(
                                        dimension_key.clone()
                                    )),
                                    related!(
                                        TaskDependencies
                                            [GameStateTransitionTask(GameState::Dimension)]
                                    ),
                                )]
                            )
                        )]
                    ),
                ));
            }
            DimensionEntryContext::GatewayTravel | DimensionEntryContext::PortalTravel => {
                todo!();
            }
        },
        Load::EnterMainMenu(MainMenuEntryContext::Init) => {
            commands.spawn((
                ReloadMetadataTask,
                related!(TaskDependencies[
                    MountPayloadSourceTask(AssetSourceId::Default),
                    FetchPayloadScopeTask(PayloadScopeId::Global),
                ]),
            ));
        }
        Load::EnterMainMenu(context) => {
            let transition_and_unload = (
                GameStateTransitionTask(GameState::MainMenu),
                related!(
                    TaskDependencies[(
                        ReloadMetadataTask,
                        related!(
                            TaskDependencies[EvictPayloadScopeTask(PayloadScopeId::Dimension(
                                active_dimension
                                    .as_ref()
                                    .expect("Can't exit world when there is no active dimension")
                                    .key()
                                    .clone(),
                            ))]
                        )
                    )]
                ),
            );
            match context {
                MainMenuEntryContext::ExitWorld => commands.spawn(transition_and_unload),
                MainMenuEntryContext::SaveAndExitWorld => commands.spawn((
                    transition_and_unload,
                    related!(TaskDependencies[WorldSavingTask]),
                )),
                MainMenuEntryContext::Init => unreachable!(),
            };
        }
        Load::Reload => {
            commands.spawn((
                ReloadMetadataTask,
                related!(TaskDependencies[RefetchPayloadTask]),
            ));
        }
    }
    commands.trigger(InitializeTasks);
    drop(entered);
    loading_span.0 = Some(span);
}

fn init_tasks(
    init_tasks: On<InitializeTasks>,
    loading_span: Res<LoadingSpan>,
    mut commands: Commands,
    mut pending_task_count: ResMut<PendingTaskCount>,
    tasks: Query<(Entity, Option<&TaskDependencies>), With<LoadingTask>>,
) {
    let _entered = loading_span.enter();
    let InitializeTasks = *init_tasks;
    pending_task_count.0 = tasks.iter().len();
    for (task, dependencies) in &tasks {
        match dependencies {
            Some(TaskDependencies(dependencies)) => {
                commands
                    .entity(task)
                    .insert(RemainingTaskDependencyCount(dependencies.len()));
            }
            None => {
                commands.trigger(BeginTask(task));
            }
        }
    }
}

fn begin_task<T: LoadingTaskComponent>(
    task_beginning: On<BeginTask>,
    loading_span: Res<LoadingSpan>,
    mut commands: Commands,
    pending_tasks: Query<&T>,
) {
    let _entered = loading_span.enter();
    let BeginTask(task) = *task_beginning;
    if let Ok(pending_task) = pending_tasks.get(task) {
        commands.queue(pending_task.task());
        debug!(task = ?pending_task, "Begin task");
    }
}

fn complete_task<T: LoadingTaskComponent, Completion: Event, Data: ReadOnlyQueryData + 'static>(
    completes_task: fn(&Completion, <Data as QueryData>::Item<'_, '_>) -> bool,
) -> impl ObserverSystem<Completion, ()> {
    IntoSystem::into_system(
        move |task_completion: On<Completion>,
              mut loading_span: ResMut<LoadingSpan>,
              mut commands: Commands,
              mut pending_task_count: ResMut<PendingTaskCount>,
              loading_tasks: Query<(Entity, Option<&TaskDependent>, &T, Data)>,
              mut remaining_dependency_count: Query<&mut RemainingTaskDependencyCount>,
              loading_overlay: Res<LoadingOverlay>,
              mut next_overlay: ResMut<NextState<ActiveOverlay>>| {
            let entered = loading_span.enter();
            let task_completion = &*task_completion;
            for (task, dependent, loading_task, task_data) in &loading_tasks {
                if !completes_task(task_completion, task_data) {
                    continue;
                }
                debug!(task = ?loading_task, "Task complete");
                commands.entity(task).despawn();
                pending_task_count.0 -= 1;
                if let Some(&TaskDependent(parent)) = dependent {
                    if let Ok(mut dependency_count) = remaining_dependency_count.get_mut(parent) {
                        dependency_count.0 -= 1;
                        if dependency_count.0 == 0 {
                            commands.trigger(BeginTask(parent));
                        }
                    }
                }
            }
            if pending_task_count.0 == 0 {
                next_overlay.set(loading_overlay.0);
                info!("Loading complete");
                drop(entered);
                loading_span.0.take().unwrap();
            }
        },
    )
}

#[derive(Event)]
struct InstantTaskCompletion<T: LoadingTaskComponent>(Entity, PhantomData<T>);

fn trigger_instant_task_completion<T: LoadingTaskComponent>(
    begin_task: On<BeginTask>,
    loading_span: Res<LoadingSpan>,
    mut commands: Commands,
    tasks: Query<(), With<T>>,
) {
    let _entered = loading_span.enter();
    let &BeginTask(begin_task) = &*begin_task;
    if tasks.contains(begin_task) {
        commands.trigger(InstantTaskCompletion::<T>(begin_task, PhantomData));
    }
}

#[inline]
fn complete_instant_task<T: LoadingTaskComponent>()
-> impl ObserverSystem<InstantTaskCompletion<T>, ()> {
    complete_task::<T, InstantTaskCompletion<T>, Entity>(
        |&InstantTaskCompletion(completed_task, _marker), task| completed_task == task,
    )
}

#[derive(Resource)]
struct LoadingScreenSettings {
    load_tip: Option<String>,
    target_tip: Option<String>,
    background: Option<()>,
}

fn init(mut commands: Commands, mut settings: ResMut<LoadingScreenSettings>) {
    commands.spawn_scene(bsn! {
        DespawnOnExit<ActiveOverlay>(ActiveOverlay::LoadingScreen)
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        Children [
            (
                Node {
                    position_type: PositionType::Absolute,
                    height: px(32),
                    left: px(2),
                    bottom: px(2),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Start,
                    align_items: AlignItems::Center,
                }
                Children [
                    ({ settings.load_tip.take().map(|tip| text(tip, WHITE, 14.)) }),
                    ({ settings.target_tip.take().map(|tip| text(tip, YELLOW, 14.)) }),
                ]
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    height: px(32),
                    right: px(2),
                    bottom: px(2),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::End,
                    align_items: AlignItems::Center,
                }
                Children [
                    (
                        Node {
                            width: px(32),
                            height: px(32),
                        }
                        ui_image_node("loading_indicator")
                    ),
                ]
            ),
        ]
    });
}

fn fina() {}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<LoadingSpan>()
        .init_resource::<LoadingOverlay>()
        .init_resource::<PendingTaskCount>()
        .add_observer(begin_loading)
        .add_observer(init_tasks)
        .add_observer(begin_task::<GameStateTransitionTask>)
        .add_observer(trigger_instant_task_completion::<GameStateTransitionTask>)
        .add_observer(complete_instant_task::<GameStateTransitionTask>())
        .add_observer(begin_task::<DimensionGenerationTask>)
        .add_observer(trigger_instant_task_completion::<DimensionGenerationTask>)
        .add_observer(complete_instant_task::<DimensionGenerationTask>())
        .add_observer(begin_task::<WorldSavingTask>)
        .add_observer(trigger_instant_task_completion::<WorldSavingTask>)
        .add_observer(complete_instant_task::<WorldSavingTask>())
        .add_observer(begin_task::<FetchPayloadScopeTask>)
        .add_observer(complete_task::<
            FetchPayloadScopeTask,
            PayloadFetchingComplete,
            (),
        >(|PayloadFetchingComplete, ()| true))
        .add_observer(begin_task::<MountPayloadSourceTask>)
        .add_observer(complete_task::<
            MountPayloadSourceTask,
            PayloadFetchingComplete,
            (),
        >(|PayloadFetchingComplete, ()| true))
        .add_observer(begin_task::<RefetchPayloadTask>)
        .add_observer(complete_task::<
            RefetchPayloadTask,
            PayloadFetchingComplete,
            (),
        >(|PayloadFetchingComplete, ()| true))
        .add_observer(begin_task::<EvictPayloadScopeTask>)
        .add_observer(trigger_instant_task_completion::<EvictPayloadScopeTask>)
        .add_observer(complete_instant_task::<EvictPayloadScopeTask>())
        .add_observer(begin_task::<UnmountPayloadSourceTask>)
        .add_observer(trigger_instant_task_completion::<UnmountPayloadSourceTask>)
        .add_observer(complete_instant_task::<UnmountPayloadSourceTask>())
        .add_observer(begin_task::<ReloadMetadataTask>)
        .add_observer(trigger_instant_task_completion::<ReloadMetadataTask>)
        .add_observer(complete_instant_task::<ReloadMetadataTask>())
        .add_systems(OnEnter(ActiveOverlay::LoadingScreen), init)
        .add_systems(OnExit(ActiveOverlay::LoadingScreen), fina);
}
