use std::time::Duration;

use freya::{animation::*, prelude::*, router::Routable};
use reclaw_input::UiMode;

use super::{
    Route,
    motion::PageMotion,
    stage_state::StageState,
    transition::{Direction, Role, Spec, TransitionConfig, frame, resolve},
    use_nav,
};

/// Level of the leaf page in the route tree: `AppLayout` is level 0 and every page sits directly under it.
const PAGE_LEVEL: usize = 1;

/// Shows the current page and animates to the next. Drop it where the pages go.
///
/// On a route change it keeps the old page, asks `resolve` how to get from it to the new one, plays
/// that over the transition's length, then drops the old page. The new page can read how far along
/// it is with `use_page_motion`.
#[derive(Clone, PartialEq)]
pub struct RouteStage {
    pub config: TransitionConfig,
    pub mode: UiMode,
}

impl Component for RouteStage {
    fn render(&self) -> impl IntoElement {
        let nav = use_nav();
        let route = nav.current();
        let mut stage = use_state({
            let route = route.clone();
            move || StageState::settled(route, Spec::none(Direction::Forward))
        });

        // A write during render is how Freya's own AnimatedRouter starts a transition: the change is
        // visible in this very frame, so there is no frame of the new route without its transition.
        if stage.peek().to != route {
            let spec = resolve(&self.config, self.mode, &stage.peek().to, &route, nav.direction());
            stage.write().go(route.clone(), spec);
        }
        let state = stage.read().clone();

        let animation = use_animation_with_dependencies(&(state.generation, state.spec.duration.as_millis() as u64), |conf, (_, ms)| {
            conf.on_creation(OnCreation::Finish);
            conf.on_change(OnChange::Rerun);
            AnimNum::new(0., 1.).duration(Duration::from_millis(*ms)).ease(Ease::Out).function(Function::Expo)
        });
        let running = animation.is_running();
        use_side_effect(move || {
            if !*running.read() && stage.peek().is_animating() {
                stage.write().settle();
            }
        });

        let progress = if state.is_animating() { animation.get().value() } else { 1. };
        rect()
            .expanded()
            .overflow(Overflow::Clip)
            .maybe_child(state.from.clone().map(|from| {
                let key = from.to_string();
                PageSlot { route: from, role: Role::Leaving, spec: state.spec, progress, key: DiffKey::None }.key(key)
            }))
            .child({
                let key = state.to.to_string();
                PageSlot { route: state.to, role: Role::Entering, spec: state.spec, progress, key: DiffKey::None }.key(key)
            })
    }
}

/// One page at one moment of a transition: moves it, fades it, and tells it where it is.
#[derive(Clone, PartialEq)]
struct PageSlot {
    route: Route,
    role: Role,
    spec: Spec,
    progress: f32,
    key: DiffKey,
}

impl KeyExt for PageSlot {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for PageSlot {
    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }

    fn render(&self) -> impl IntoElement {
        let mut motion = use_state(|| PageMotion::REST);
        use_provide_context(move || motion);
        motion.set_if_modified(PageMotion::new(self.role, self.progress, &self.spec));

        let f = frame(&self.spec, self.role, self.progress);
        rect()
            .position(Position::new_absolute().top(0.).left(0.))
            .width(Size::fill())
            .height(Size::fill())
            .opacity(f.opacity)
            .offset_x(f.dx)
            .offset_y(f.dy)
            .scale(f.scale)
            // The page on its way out cannot be pressed.
            .interactive(if self.role == Role::Entering { Interactive::Yes } else { Interactive::No })
            .child(self.route.render(PAGE_LEVEL))
    }
}
