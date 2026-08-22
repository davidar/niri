use crate::utils::MergeWith;
use crate::FloatOrInt;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Gestures {
    pub dnd_edge_view_scroll: DndEdgeViewScroll,
    pub dnd_edge_workspace_switch: DndEdgeWorkspaceSwitch,
    pub hot_corners: HotCorners,
    pub touchpad_swipe: TouchpadSwipe,
    pub pointer_drag: PointerDrag,
    pub touch: TouchGestures,
}

#[derive(knuffel::Decode, Debug, Default, Clone, Copy, PartialEq)]
pub struct GesturesPart {
    #[knuffel(child)]
    pub dnd_edge_view_scroll: Option<DndEdgeViewScrollPart>,
    #[knuffel(child)]
    pub dnd_edge_workspace_switch: Option<DndEdgeWorkspaceSwitchPart>,
    #[knuffel(child)]
    pub hot_corners: Option<HotCorners>,
    #[knuffel(child)]
    pub touchpad_swipe: Option<TouchpadSwipePart>,
    #[knuffel(child)]
    pub pointer_drag: Option<PointerDragPart>,
    #[knuffel(child)]
    pub touch: Option<TouchGesturesPart>,
}

impl MergeWith<GesturesPart> for Gestures {
    fn merge_with(&mut self, part: &GesturesPart) {
        merge!(
            (self, part),
            dnd_edge_view_scroll,
            dnd_edge_workspace_switch,
            touchpad_swipe,
            pointer_drag,
            touch,
        );
        merge_clone!((self, part), hot_corners);
    }
}

/// Rubber-band overscroll parameters (see `RubberBand` in niri).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RubberBandParams {
    pub stiffness: f64,
    pub limit: f64,
}

#[derive(knuffel::Decode, Debug, Clone, Copy, PartialEq)]
pub struct RubberBandPart {
    #[knuffel(property)]
    pub stiffness: Option<FloatOrInt<0, 1000>>,
    #[knuffel(property)]
    pub limit: Option<FloatOrInt<0, 1000>>,
}

impl MergeWith<RubberBandPart> for RubberBandParams {
    fn merge_with(&mut self, part: &RubberBandPart) {
        merge!((self, part), stiffness, limit);
    }
}

/// Mod+mouse-drag gestures (workspace switch and view scroll).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerDrag {
    /// Pointer travel before the drag locks to a horizontal or vertical gesture.
    pub direction_lock_distance: f64,
}

impl Default for PointerDrag {
    fn default() -> Self {
        Self {
            direction_lock_distance: 8., // Taken from GTK 4.
        }
    }
}

#[derive(knuffel::Decode, Debug, Clone, Copy, PartialEq)]
pub struct PointerDragPart {
    #[knuffel(child, unwrap(argument))]
    pub direction_lock_distance: Option<FloatOrInt<0, 65535>>,
}

impl MergeWith<PointerDragPart> for PointerDrag {
    fn merge_with(&mut self, part: &PointerDragPart) {
        merge!((self, part), direction_lock_distance);
    }
}

/// Touchscreen gestures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchGestures {
    /// How long a touch must stay still in the overview before it starts moving the window.
    pub long_press_ms: u16,
}

impl Default for TouchGestures {
    fn default() -> Self {
        Self { long_press_ms: 500 }
    }
}

#[derive(knuffel::Decode, Debug, Clone, Copy, PartialEq)]
pub struct TouchGesturesPart {
    #[knuffel(child, unwrap(argument))]
    pub long_press_ms: Option<u16>,
}

impl MergeWith<TouchGesturesPart> for TouchGestures {
    fn merge_with(&mut self, part: &TouchGesturesPart) {
        merge_clone!((self, part), long_press_ms);
    }
}

/// Touchpad swipe distances: how far the fingers travel for one unit of the gesture.
///
/// A gesture commits to the nearest unit when released (plus velocity), so these also set
/// how far a slow swipe must go before it no longer snaps back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchpadSwipe {
    /// Movement to switch the height of one workspace (three-finger vertical swipe).
    pub workspace_movement: f64,
    /// Movement to scroll the view by the width of one working area (three-finger
    /// horizontal swipe).
    pub view_movement: f64,
    /// Movement to fully open or close the overview (four-finger vertical swipe).
    pub overview_movement: f64,
    /// Finger count for the workspace-switch / view-scroll swipe.
    pub workspace_fingers: u32,
    /// Finger count for the overview swipe.
    pub overview_fingers: u32,
    /// Per-millisecond velocity retention used to project where a released swipe coasts to.
    pub deceleration: f64,
    /// How much recent movement (ms) feeds the release velocity estimate.
    pub velocity_window_ms: u16,
    /// Overscroll feel past the first/last workspace.
    pub workspace_rubber_band: RubberBandParams,
    /// Overscroll feel past the overview's open/closed ends.
    pub overview_rubber_band: RubberBandParams,
}

impl Default for TouchpadSwipe {
    fn default() -> Self {
        Self {
            workspace_movement: 300.,
            view_movement: 1200.,
            overview_movement: 300.,
            workspace_fingers: 3,
            overview_fingers: 4,
            deceleration: 0.997,
            velocity_window_ms: 150,
            workspace_rubber_band: RubberBandParams {
                stiffness: 0.5,
                limit: 0.05,
            },
            overview_rubber_band: RubberBandParams {
                stiffness: 0.5,
                limit: 0.05,
            },
        }
    }
}

#[derive(knuffel::Decode, Debug, Clone, Copy, PartialEq)]
pub struct TouchpadSwipePart {
    #[knuffel(child, unwrap(argument))]
    pub workspace_movement: Option<FloatOrInt<1, 65535>>,
    #[knuffel(child, unwrap(argument))]
    pub view_movement: Option<FloatOrInt<1, 65535>>,
    #[knuffel(child, unwrap(argument))]
    pub overview_movement: Option<FloatOrInt<1, 65535>>,
    #[knuffel(child, unwrap(argument))]
    pub workspace_fingers: Option<u32>,
    #[knuffel(child, unwrap(argument))]
    pub overview_fingers: Option<u32>,
    #[knuffel(child, unwrap(argument))]
    pub deceleration: Option<FloatOrInt<0, 1>>,
    #[knuffel(child, unwrap(argument))]
    pub velocity_window_ms: Option<u16>,
    #[knuffel(child)]
    pub workspace_rubber_band: Option<RubberBandPart>,
    #[knuffel(child)]
    pub overview_rubber_band: Option<RubberBandPart>,
}

impl MergeWith<TouchpadSwipePart> for TouchpadSwipe {
    fn merge_with(&mut self, part: &TouchpadSwipePart) {
        merge!(
            (self, part),
            workspace_movement,
            view_movement,
            overview_movement,
            deceleration,
            workspace_rubber_band,
            overview_rubber_band,
        );
        merge_clone!(
            (self, part),
            workspace_fingers,
            overview_fingers,
            velocity_window_ms,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DndEdgeViewScroll {
    pub trigger_width: f64,
    pub delay_ms: u16,
    pub max_speed: f64,
}

impl Default for DndEdgeViewScroll {
    fn default() -> Self {
        Self {
            trigger_width: 30., // Taken from GTK 4.
            delay_ms: 100,
            max_speed: 1500.,
        }
    }
}

#[derive(knuffel::Decode, Debug, Clone, Copy, PartialEq)]
pub struct DndEdgeViewScrollPart {
    #[knuffel(child, unwrap(argument))]
    pub trigger_width: Option<FloatOrInt<0, 65535>>,
    #[knuffel(child, unwrap(argument))]
    pub delay_ms: Option<u16>,
    #[knuffel(child, unwrap(argument))]
    pub max_speed: Option<FloatOrInt<0, 1_000_000>>,
}

impl MergeWith<DndEdgeViewScrollPart> for DndEdgeViewScroll {
    fn merge_with(&mut self, part: &DndEdgeViewScrollPart) {
        merge!((self, part), trigger_width, max_speed);
        merge_clone!((self, part), delay_ms);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DndEdgeWorkspaceSwitch {
    pub trigger_height: f64,
    pub delay_ms: u16,
    pub max_speed: f64,
    /// Edge-scroll distance equivalent to one workspace height; 1500 matches the default
    /// max-speed (one screen height per second).
    pub workspace_movement: f64,
}

impl Default for DndEdgeWorkspaceSwitch {
    fn default() -> Self {
        Self {
            trigger_height: 50.,
            delay_ms: 100,
            max_speed: 1500.,
            workspace_movement: 1500.,
        }
    }
}

#[derive(knuffel::Decode, Debug, Clone, Copy, PartialEq)]
pub struct DndEdgeWorkspaceSwitchPart {
    #[knuffel(child, unwrap(argument))]
    pub trigger_height: Option<FloatOrInt<0, 65535>>,
    #[knuffel(child, unwrap(argument))]
    pub delay_ms: Option<u16>,
    #[knuffel(child, unwrap(argument))]
    pub max_speed: Option<FloatOrInt<0, 1_000_000>>,
    #[knuffel(child, unwrap(argument))]
    pub workspace_movement: Option<FloatOrInt<1, 1_000_000>>,
}

impl MergeWith<DndEdgeWorkspaceSwitchPart> for DndEdgeWorkspaceSwitch {
    fn merge_with(&mut self, part: &DndEdgeWorkspaceSwitchPart) {
        merge!((self, part), trigger_height, max_speed, workspace_movement);
        merge_clone!((self, part), delay_ms);
    }
}

#[derive(knuffel::Decode, Debug, Default, Clone, Copy, PartialEq)]
pub struct HotCorners {
    #[knuffel(child)]
    pub off: bool,
    #[knuffel(child)]
    pub top_left: bool,
    #[knuffel(child)]
    pub top_right: bool,
    #[knuffel(child)]
    pub bottom_left: bool,
    #[knuffel(child)]
    pub bottom_right: bool,
    /// Size of the corner trigger area in logical pixels (default 1).
    #[knuffel(child, unwrap(argument))]
    pub trigger_size: Option<FloatOrInt<1, 65535>>,
}
