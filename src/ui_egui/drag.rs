use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime};
use egui::{Context, Id, Pos2, Rect, Vec2};

use crate::models::event::Event;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragView {
    Day,
    Week,
    WorkWeek,
}

#[derive(Clone, Debug)]
pub struct DragContext {
    pub event_id: i64,
    pub original_start: DateTime<Local>,
    #[allow(dead_code)]
    pub original_end: DateTime<Local>,
    pub duration: Duration,
    #[allow(dead_code)]
    pub pointer_offset: Vec2,
    pub pointer_pos: Option<Pos2>,
    pub hovered_date: Option<NaiveDate>,
    pub hovered_time: Option<NaiveTime>,
    pub hovered_rect: Option<Rect>,
    pub view: DragView,
}

impl DragContext {
    pub fn from_event(event: &Event, pointer_offset: Vec2, view: DragView) -> Option<Self> {
        let event_id = event.id?;
        Some(Self {
            event_id,
            duration: event.end - event.start,
            original_start: event.start,
            original_end: event.end,
            pointer_offset,
            pointer_pos: None,
            hovered_date: Some(event.start.date_naive()),
            hovered_time: Some(event.start.time()),
            hovered_rect: None,
            view,
        })
    }

    pub fn hovered_start(&self) -> Option<DateTime<Local>> {
        match (self.hovered_date, self.hovered_time) {
            (Some(date), Some(time)) => date.and_time(time).and_local_timezone(Local).single(),
            _ => None,
        }
    }
}

pub struct DragManager;

impl DragManager {
    fn storage_id() -> Id {
        Id::new("calendar_event_drag_state")
    }

    pub fn begin(ctx: &Context, context: DragContext) {
        ctx.memory_mut(|mem| {
            mem.data.insert_persisted(Self::storage_id(), context);
        });
    }

    pub fn active(ctx: &Context) -> Option<DragContext> {
        ctx.memory_mut(|mem| mem.data.get_persisted::<DragContext>(Self::storage_id()))
    }

    pub fn active_for_view(ctx: &Context, view: DragView) -> Option<DragContext> {
        Self::active(ctx).filter(|ctx_data| ctx_data.view == view)
    }

    pub fn is_active_for_view(ctx: &Context, view: DragView) -> bool {
        Self::active_for_view(ctx, view).is_some()
    }

    pub fn update_hover(
        ctx: &Context,
        date: NaiveDate,
        time: NaiveTime,
        rect: Rect,
        pointer_pos: Pos2,
    ) {
        let id = Self::storage_id();
        ctx.memory_mut(|mem| {
            if let Some(mut state) = mem.data.get_persisted::<DragContext>(id) {
                state.hovered_date = Some(date);
                state.hovered_time = Some(time);
                state.hovered_rect = Some(rect);
                state.pointer_pos = Some(pointer_pos);
                mem.data.insert_persisted(id, state);
            }
        });
    }

    pub fn finish_for_view(ctx: &Context, view: DragView) -> Option<DragContext> {
        let id = Self::storage_id();
        let mut result = None;
        ctx.memory_mut(|mem| {
            if let Some(current) = mem.data.get_persisted::<DragContext>(id) {
                if current.view == view {
                    result = Some(current);
                    mem.data.remove::<DragContext>(id);
                }
            }
        });
        result
    }

    #[allow(dead_code)]
    pub fn cancel(ctx: &Context) {
        ctx.memory_mut(|mem| {
            mem.data.remove::<DragContext>(Self::storage_id());
        });
    }

    #[allow(dead_code)]
    pub fn cancel_for_view(ctx: &Context, view: DragView) {
        let id = Self::storage_id();
        ctx.memory_mut(|mem| {
            if let Some(current) = mem.data.get_persisted::<DragContext>(id) {
                if current.view == view {
                    mem.data.remove::<DragContext>(id);
                }
            }
        });
    }
}
