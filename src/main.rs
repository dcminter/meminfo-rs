use gtk::glib::{ControlFlow, ExitCode};
use gtk::prelude::*;
use gtk::{Align, Application, ApplicationWindow, GestureClick, Grid, Label, LevelBar};
use std::iter::zip;

mod copies;
use copies::CopyStatus;

mod buffers;
use buffers::{MemCounts, MemRange};

const READER_FREQUENCY_SECONDS: u32 = 2;

fn main() -> ExitCode {
    let app = Application::builder()
        .application_id("org.paperstack.Meminfo")
        .build();
    app.connect_activate(on_activate);
    app.run()
}

struct LabelledProgressBar {
    label: Label,
    bar: LevelBar,
    text: Label,
    numeric: Label,
}

fn on_activate(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Memory & File Information")
        .show_menubar(true)
        .build();

    // UI elements for the "Dirty" value
    let dirty = LabelledProgressBar {
        label: Label::new(Some("Dirty")),
        bar: LevelBar::new(),
        text: Label::new(None),
        numeric: Label::new(None),
    };
    dirty.bar.set_hexpand(true);

    // UI elements for the "Writeback" value
    let writeback = LabelledProgressBar {
        label: Label::new(Some("Writeback")),
        bar: LevelBar::new(),
        text: Label::new(None),
        numeric: Label::new(None),
    };
    writeback.bar.set_hexpand(true);

    // Attach controllers to the level bars (not being used though currently)
    attach_controllers(&dirty.bar, &writeback.bar);

    // Build the UI layout
    let mut grid = build_layout(&dirty, &writeback);
    window.set_child(Some(&grid));

    let mut mem_counts = MemCounts {
        dirty: MemRange {
            current: 0,
            highest: 0,
        },
        writeback: MemRange {
            current: 0,
            highest: 0,
        },
    };

    let mut file_labels: Vec<LabelledProgressBar> = vec![];

    // Run before attempting to render anything to ensure we have initial values set nicely
    update_all_level_bars(
        &mut grid,
        &mut mem_counts,
        &dirty,
        &writeback,
        &mut file_labels,
    );

    // Then schedule to run every few seconds to update the bars.
    gtk::glib::timeout_add_seconds_local(READER_FREQUENCY_SECONDS, move || {
        update_all_level_bars(
            &mut grid,
            &mut mem_counts,
            &dirty,
            &writeback,
            &mut file_labels,
        );
        ControlFlow::Continue
    });

    // Off to the races...
    window.present();
}

fn attach_controllers(dirty_level_bar: &LevelBar, writeback_level_bar: &LevelBar) {
    let dirty_level_bar_click = GestureClick::new();
    dirty_level_bar_click.connect_pressed(|_, _, _, _| {
        // I'm not actually using these yet, but I do intend to at some point...
        println!("CLICKED DIRTY");
    });
    dirty_level_bar.add_controller(dirty_level_bar_click);

    let writeback_level_bar_click = GestureClick::new();
    writeback_level_bar_click.connect_pressed(|_, _, _, _| {
        // I'm not actually using these yet, but I do intend to at some point...
        println!("CLICKED WRITEBACK");
    });
    writeback_level_bar.add_controller(writeback_level_bar_click);
}

fn build_layout(dirty: &LabelledProgressBar, writeback: &LabelledProgressBar) -> Grid {
    let grid = Grid::new();
    grid.set_column_spacing(5);
    grid.set_row_spacing(5);
    grid.set_margin_top(5);
    grid.set_margin_bottom(5);
    grid.set_margin_start(5);
    grid.set_margin_end(5);

    grid.attach(&dirty.label, 0, 0, 1, 1);
    grid.attach(&dirty.bar, 1, 0, 1, 1);
    grid.attach(&dirty.numeric, 2, 0, 1, 1);

    grid.attach(&writeback.label, 0, 1, 1, 1);
    grid.attach(&writeback.bar, 1, 1, 1, 1);
    grid.attach(&writeback.numeric, 2, 1, 1, 1);

    grid
}

fn update_buffer_level(range: &MemRange, labelled_progress_bar: &LabelledProgressBar) {
    labelled_progress_bar.bar.set_value(range.current as f64);
    labelled_progress_bar
        .bar
        .set_max_value(range.highest as f64);

    // This is potentially sketchy; I'm assuming the units are always kib because the actual kernel
    // code as it currently stands never returns anything other than kib (kb)

    // kib to bytes (assuming kib units)
    let converted = human_bytes::human_bytes(range.current as f64);

    labelled_progress_bar
        .numeric
        .set_label(format!("{}", converted).as_str());
}

fn update_file_copy_progress_rows(
    grid: &mut Grid,
    statuses: &Vec<CopyStatus>,
    file_labels: &mut Vec<LabelledProgressBar>,
) {
    if file_labels.len() < statuses.len() {
        // Create new ones
        for _ in 0..(statuses.len() - file_labels.len()) {
            let file_meter = LabelledProgressBar {
                label: Label::new(None),
                bar: LevelBar::new(),
                text: Label::new(None),
                numeric: Label::new(None),
            };

            file_meter.bar.set_hexpand(true);
            file_meter.text.set_hexpand(true);
            file_meter.text.set_halign(Align::Center);

            let _ = file_meter.label.set_tooltip_text(Some("PID"));
            let _ = file_meter.numeric.set_tooltip_text(Some("Completion"));

            let row_offset = (file_labels.len() as i32) + 2;
            grid.attach(&file_meter.label, 0, row_offset, 1, 1);
            grid.attach(&file_meter.bar, 1, row_offset, 1, 1);
            grid.attach(&file_meter.text, 1, row_offset, 1, 1); // Draw this text widget on top of the bar widget...
            grid.attach(&file_meter.numeric, 2, row_offset, 1, 1);

            file_labels.push(file_meter);
        }
    } else if file_labels.len() > statuses.len() {
        // Remove these labels from the layout box!
        for deletion in &file_labels[statuses.len()..] {
            grid.remove(&deletion.label);
            grid.remove(&deletion.bar);
            grid.remove(&deletion.text);
            grid.remove(&deletion.numeric);
        }

        // Now delete the rows themselves
        for last_row_index in ((statuses.len() as i32) + 2)..2 {
            grid.remove_row(last_row_index);
        }

        // This should get them deleted completely (I hope)
        file_labels.truncate(statuses.len());
    }

    zip(statuses, file_labels).for_each(|(status, file_label)| {
        update_file_copy_pid_label(status, &file_label);
        update_file_progress_bar(&status, &file_label);
        update_file_progress_label(&status, &file_label);
    });
}

fn update_file_progress_label(status: &&CopyStatus, file_label: &&mut LabelledProgressBar) {
    let percentage = copies::calculate_percentage_of_completion(&status);
    let right = format!("[{:.0}%]", percentage);
    let _ = &file_label.numeric.set_label(right.as_str());
}

fn update_file_progress_bar(status: &&CopyStatus, file_label: &&mut LabelledProgressBar) {
    let (maximum, actual) = match &status.source {
        Some(source) => (source.length, source.offset),
        None => (0, 0),
    };

    let _ = &file_label.bar.set_value(actual as f64);
    let _ = &file_label.bar.set_max_value(maximum as f64);

    let source_filename = copies::get_filename_lossy(&status.source);
    let target_filename = copies::get_filename_lossy(&status.target);
    let tooltip = format!("Source {} to target {}", source_filename, target_filename);

    let _ = file_label.bar.set_tooltip_text(Some(tooltip.as_str()));
    let _ = file_label.text.set_text(source_filename.as_str());
    let _ = file_label.text.set_tooltip_text(Some(tooltip.as_str()));
}

fn update_file_copy_pid_label(status: &CopyStatus, file_label: &&mut LabelledProgressBar) {
    let left = format!("{}", status.pid);
    let _ = &file_label.label.set_label(left.as_str());
}

fn update_all_level_bars(
    grid: &mut Grid,
    mc: &mut MemCounts,
    dirty: &LabelledProgressBar,
    writeback: &LabelledProgressBar,
    file_labels: &mut Vec<LabelledProgressBar>,
) {
    // Read and update the buffer level bars
    buffers::meminfo_reader(mc);
    update_buffer_level(&mc.dirty, &dirty);
    update_buffer_level(&mc.writeback, &writeback);

    // Read and update the file copy progress bars (creating/deleting them as necessary)
    let statuses = copies::file_copy_info();
    match statuses {
        Ok(statuses) => {
            update_file_copy_progress_rows(grid, &statuses, file_labels);
        }
        Err(_) => {
            // There are various races etc. going on and this is a fluffy operation at best; we're
            // going to ignore everything. One example is that getting the metadata can race the
            // listing of processes, so by the time we're ready to read the metadata the process
            // doesn't exist and we'd get a File not found. Stuff like that. If I add logging later,
            // I'll suppy a log output at a very low priority (trace/debug or whatever).
        }
    }
}
