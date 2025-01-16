mod imp;
use gtk::{glib, LayoutManager};

glib::wrapper! {
    pub struct MeminfoLayoutManager(ObjectSubclass<imp::MeminfoLayoutManager>)
        @extends LayoutManager;
}

impl MeminfoLayoutManager {}
