//! A layer-shell surface backed by a wl_shm slot pool. Drawing goes directly
//! into a reusable shared-memory slot (no intermediate pixmap); each surface
//! keeps a small pool of slots and reuses whichever the compositor released.

use smithay_client_toolkit::{
    compositor::{CompositorState, Region},
    reexports::client::{QueueHandle, protocol::wl_shm::Format},
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface},
    },
    shm::{
        Shm,
        slot::{Slot, SlotPool},
    },
};
use tiny_skia::{Color, PixmapMut};

use crate::shell::Shell;

pub struct Surface {
    pub layer: LayerSurface,
    pub pool: SlotPool,
    /// Persistent shared-memory slots. SCTK's `create_buffer` allocates a new
    /// slot on every call; reusing two slots per surface keeps RSS flat.
    slots: Vec<Slot>,
    pub width: u32,
    pub height: u32,
    pub configured: bool,
}

impl Surface {
    pub fn new(
        compositor: &CompositorState,
        layer_shell: &LayerShell,
        shm: &Shm,
        qh: &QueueHandle<Shell>,
        namespace: &str,
        layer_kind: Layer,
        size: (u32, u32),
        anchor: Anchor,
        exclusive_zone: i32,
    ) -> Self {
        let wl_surface = compositor.create_surface(qh);
        let layer = layer_shell.create_layer_surface(
            qh,
            wl_surface,
            layer_kind,
            Some(namespace.to_string()),
            None,
        );
        layer.set_anchor(anchor);
        layer.set_exclusive_zone(exclusive_zone);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_size(size.0, size.1);
        layer.wl_surface().commit();

        Self {
            layer,
            pool: SlotPool::new(256 * 256 * 4, shm).expect("slot pool"),
            slots: Vec::new(),
            width: size.0,
            height: size.1,
            configured: false,
        }
    }

    /// Applies a configure event; returns true if the size changed.
    pub fn apply_configure(&mut self, new_size: (u32, u32)) -> bool {
        let mut changed = !self.configured;
        if new_size.0 != 0 && new_size.0 != self.width {
            self.width = new_size.0;
            changed = true;
        }
        if new_size.1 != 0 && new_size.1 != self.height {
            self.height = new_size.1;
            changed = true;
        }
        self.configured = true;
        changed
    }

    /// Changes keyboard interactivity and commits so the compositor applies it.
    pub fn set_keyboard(&self, mode: KeyboardInteractivity) {
        self.layer.set_keyboard_interactivity(mode);
        self.layer.wl_surface().commit();
    }

    /// Replaces the contents with a fully transparent buffer and frees the
    /// pixmap. The surface stays mapped: niri downgrades a layer surface to
    /// the Background layer if it is unmapped via a null commit and remapped.
    pub fn clear_transparent(&mut self) {
        self.redraw(|_| {});
    }

    /// Makes the surface transparent to input (click-through) or restores the
    /// default infinite input region.
    pub fn set_click_through(&self, empty: &Region, enabled: bool) {
        let wl = self.layer.wl_surface();
        wl.set_input_region(if enabled { Some(empty.wl_region()) } else { None });
        wl.commit();
    }

    /// Draws directly into a shared-memory slot and commits the buffer.
    pub fn redraw<F>(&mut self, draw: F)
    where
        F: FnOnce(&mut PixmapMut),
    {
        if self.width == 0 || self.height == 0 || !self.configured {
            return;
        }
        let (w, h) = (self.width, self.height);
        let Self { pool, slots, layer, .. } = self;
        let stride = w as i32 * 4;
        let needed = (h * stride as u32) as usize;

        // Reuse a released slot; allocate another only while the compositor
        // still holds the previous frame (steady state: two slots).
        let mut slot_idx = slots
            .iter()
            .position(|s| s.len() >= needed && !s.has_active_buffers());
        if slot_idx.is_none() {
            let slot = pool.new_slot(needed).expect("alloc shm slot");
            slots.push(slot);
            slot_idx = Some(slots.len() - 1);
        }
        let idx = slot_idx.unwrap();

        let canvas = slots[idx].canvas(pool).expect("free shm slot canvas");
        let mut pm = PixmapMut::from_bytes(&mut canvas[..needed], w, h)
            .expect("pixmap over shm slot");
        pm.fill(Color::TRANSPARENT);
        draw(&mut pm);

        let buffer = pool
            .create_buffer_in(&slots[idx], w as i32, h as i32, stride, Format::Abgr8888)
            .expect("create shm buffer");
        let wl = layer.wl_surface();
        buffer.attach_to(wl).expect("attach shm buffer");
        wl.damage_buffer(0, 0, w as i32, h as i32);
        wl.commit();
    }
}
