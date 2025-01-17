use gtk::glib::ControlFlow;
use gtk::prelude::*;
use gtk::{
    glib, Align, Application, ApplicationWindow, FlowBox, GestureClick, Label, LevelBar, ListBox,
    SelectionMode,
};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::string::ToString;

const PROC_MEMINFO_PATH: &str = "/proc/meminfo";
const READER_FREQUENCY_SECONDS: u32 = 2;
const MEMINFO_LINE_PATTERN: &str =
    "([[:alpha:]]+):[[:space:]]+([[:digit:]]+)[[:space:]]+([[:alpha:]]+)";
const MEMINFO_KEY_DIRTY: &str = "Dirty";
const MEMINFO_KEY_WRITEBACK: &str = "Writeback";

/// The latest progress of a cache entry
#[derive(Debug)]
struct MemRange {
    /// The current value of the cache entry
    current: f64,

    /// The highest value seen for the cache entry so far
    highest: f64,

    /// The units in which the values are expressed, expected to be 'kB'
    units: String,
}

/// The cache entries that this utility is tracking
#[derive(Debug)]
struct MemCounts {
    /// Memory which is waiting to get written back to the disk
    dirty: MemRange,

    /// Memory which is actively being written back to the disk
    writeback: MemRange,
}

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id("org.paperstack.Meminfo")
        .build();
    app.connect_activate(on_activate);
    app.run()
}

#[test]
fn test_memory_count_update_bump_highest_same_units() {
    let mut range = MemRange {
        current: 5.0,
        highest: 6.0,
        units: "kB".to_string(),
    };

    let entry = Some(&("12", "kB"));

    memory_count_update(entry, &mut range);

    assert_eq!(range.current, 12.0, "Current value was not set");
    assert_eq!(range.highest, 12.0, "Highest value was not bumped up");
}

#[test]
fn test_memory_count_update_no_bump_to_highest_same_units() {
    let mut range = MemRange {
        current: 5.0,
        highest: 6.0,
        units: "kB".to_string(),
    };

    let entry = Some(&("4", "kB"));

    memory_count_update(entry, &mut range);

    assert_eq!(range.current, 4.0, "Current value was not set");
    assert_eq!(range.highest, 6.0, "Highest value was incorrectly changed");
}

///   Note - this assumes that the units don't change - in practice the kernel source currently
///   has the following form with formatted print lines:
///
///   ...
///       "Dirty:      %8lu kB\n"
///   "Writeback:      %8lu kB\n"
///   ...
///
///   So the units cannot *currently* change; however I don't think it would be considered a
///   breaking change to the userspace APIs for the units reported to change dynamically in some
///   future version. I'll keep an eye on it and might re-work this to allow for it if I'm feeling
///   very keen in the future!
fn memory_count_update(entry: Option<&(&str, &str)>, range: &mut MemRange) {
    match process_parsed_meminfo_entry(entry) {
        Some((numeric, unit)) => {
            range.units = unit;
            if numeric > range.highest {
                range.current = numeric;
                range.highest = numeric;
            } else {
                range.current = numeric;
            };
        }
        None => {
            eprintln!("Memory count not found in {}", PROC_MEMINFO_PATH)
        }
    }
}

/// For a provided path (expected to be `/proc/meminfo`) this open it as a file and then use the
/// provided regular expression to scan for the `Dirty` and `Writeback` lines, and populate the
/// provided `MemCounts` structure accordingly.
fn meminfo_reader(path: &str, line_regex: &Regex, mc: &mut MemCounts) {
    match fs::read_to_string(path) {
        Ok(text) => {
            let mapped_meminfo: HashMap<&str, (&str, &str)> = line_regex
                .captures_iter(text.as_str())
                .map(|c| c.extract())
                .map(|(_, [key, value, unit])| (key, (value, unit)))
                .collect();

            memory_count_update(mapped_meminfo.get(MEMINFO_KEY_DIRTY), &mut mc.dirty);
            memory_count_update(mapped_meminfo.get(MEMINFO_KEY_WRITEBACK), &mut mc.writeback);
        }
        Err(error) => {
            eprintln!("Error reading {}: {}", PROC_MEMINFO_PATH, error);
        }
    }
}

fn process_parsed_meminfo_entry(entry: Option<&(&str, &str)>) -> Option<(f64, String)> {
    match entry {
        Some((value, unit)) => match (*value).parse::<i64>() {
            Ok(value) => {
                let units = (*unit).to_string();
                Some((value as f64, units))
            }
            Err(error) => {
                eprintln!("The numeric part ('{}') of a meminfo line could not be parsed as a numeric value: {}", value, error);
                None
            }
        },
        None => {
            eprintln!("Dirty memory count not found in {}", PROC_MEMINFO_PATH);
            None
        }
    }
}

fn on_activate(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Memory Information")
        .show_menubar(true)
        .build();

    let dirty_level_bar = LevelBar::new();
    //dirty_level_bar.set_width_request(DEFAULT_LEVEL_BAR_PIXEL_WIDTH);

    let dirty_level_bar_click = GestureClick::new();
    dirty_level_bar_click.connect_pressed(|_, _, _, _| {
        println!("CLICKED DIRTY");
    });
    dirty_level_bar.add_controller(dirty_level_bar_click);

    let writeback_level_bar = LevelBar::new();
    //writeback_level_bar.set_width_request(DEFAULT_LEVEL_BAR_PIXEL_WIDTH);

    let writeback_level_bar_click = GestureClick::new();
    writeback_level_bar_click.connect_pressed(|_, _, _, _| {
        println!("CLICKED WRITEBACK");
    });
    writeback_level_bar.add_controller(writeback_level_bar_click);

    let flow_box = FlowBox::new();
    flow_box.set_column_spacing(5);
    flow_box.set_row_spacing(5);

    // grid.set_column_homogeneous(false); // ?
    flow_box.set_margin_start(5);
    flow_box.set_margin_end(5);
    flow_box.set_margin_top(5);
    flow_box.set_margin_bottom(5);
    flow_box.set_vexpand(false);
    flow_box.set_selection_mode(SelectionMode::None);
    flow_box.set_min_children_per_line(3);
    flow_box.set_max_children_per_line(3);

    let dirty_label = Label::new(Some("Dirty"));
    dirty_label.set_halign(Align::Start);
    dirty_label.set_hexpand(false);
    flow_box.insert(&dirty_label, -1);

    let dirty_level_bar_list = ListBox::new();
    dirty_level_bar_list.set_valign(Align::Center);
    dirty_level_bar_list.insert(&dirty_level_bar, -1);
    flow_box.insert(&dirty_level_bar_list, -1);

    let dirty_numeric_label = Label::new(None);
    dirty_numeric_label.set_halign(Align::End);
    dirty_numeric_label.set_width_request(150);
    flow_box.insert(&dirty_numeric_label, -1);

    let writeback_label = Label::new(Some("Writeback"));
    writeback_label.set_halign(Align::Start);
    writeback_label.set_hexpand(false);
    flow_box.insert(&writeback_label, -1);

    let writeback_level_bar_list = ListBox::new();
    writeback_level_bar_list.set_valign(Align::Center);
    writeback_level_bar_list.insert(&writeback_level_bar, -1);
    flow_box.insert(&writeback_level_bar_list, -1);

    let writeback_numeric_label = Label::new(None);
    writeback_numeric_label.set_halign(Align::End);
    writeback_numeric_label.set_width_request(150);
    flow_box.insert(&writeback_numeric_label, -1);

    window.set_child(Some(&flow_box));

    // There's no recovering from an error here...
    let line_regex = Regex::new(MEMINFO_LINE_PATTERN)
        .expect("Failed to parse the compiled-in regular expression! Heavens above! :)");

    let mut mem_counts = MemCounts {
        dirty: MemRange {
            current: 0.0,
            highest: 0.0,
            units: "Unknown".to_string(),
        },
        writeback: MemRange {
            current: 0.0,
            highest: 0.0,
            units: "Unknown".to_string(),
        },
    };

    // Run before attempting to render anything to ensure we have initial values set nicely
    update_level_bars(
        &line_regex,
        &mut mem_counts,
        &dirty_level_bar,
        &dirty_numeric_label,
        &writeback_level_bar,
        &writeback_numeric_label,
    );

    // Then schedule to run every few seconds to update the bars.
    glib::timeout_add_seconds_local(READER_FREQUENCY_SECONDS, move || {
        update_level_bars(
            &line_regex,
            &mut mem_counts,
            &dirty_level_bar,
            &dirty_numeric_label,
            &writeback_level_bar,
            &writeback_numeric_label,
        );
        ControlFlow::Continue
    });

    // Off to the races...
    window.present();
}

fn update_level(range: &MemRange, level_bar: &LevelBar, label: &Label) {
    level_bar.set_value(range.current);
    level_bar.set_max_value(range.highest);
    label.set_label(format!("{} {}", range.current, range.units).as_str());
}

fn update_level_bars(
    line_regex: &Regex,
    mc: &mut MemCounts,
    dirty_level_bar: &LevelBar,
    dirty_numeric_label: &Label,
    writeback_level_bar: &LevelBar,
    writeback_numeric_label: &Label,
) {
    meminfo_reader(PROC_MEMINFO_PATH, &line_regex, mc);
    update_level(&mc.dirty, dirty_level_bar, dirty_numeric_label);
    update_level(&mc.writeback, writeback_level_bar, writeback_numeric_label);
}
