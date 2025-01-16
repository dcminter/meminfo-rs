use gtk::subclass::layout_manager::LayoutManagerImpl;
use gtk::subclass::prelude::{ObjectImpl, ObjectSubclass};
use gtk::{glib, LayoutManager, Orientation, SizeRequestMode, Widget};

#[derive(Debug)]
pub struct MeminfoLayoutManager {}

impl Default for MeminfoLayoutManager {
    fn default() -> Self {
        todo!()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for MeminfoLayoutManager {
    const NAME: &'static str = "MeminfoLayoutManager";
    type Type = super::MeminfoLayoutManager;
    type ParentType = LayoutManager;
}

impl ObjectImpl for MeminfoLayoutManager {}

impl LayoutManagerImpl for MeminfoLayoutManager {
    fn request_mode(&self, _widget: &Widget) -> SizeRequestMode {
        todo!()
    }

    fn measure(
        &self,
        _widget: &Widget,
        _orientation: Orientation,
        _for_size: i32,
    ) -> (i32, i32, i32, i32) {
        todo!()
    }

    fn allocate(&self, _widget: &Widget, _width: i32, _height: i32, _baseline: i32) {
        todo!()
    }
}
