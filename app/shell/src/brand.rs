//! The product name in one place (DESIGN.md §11, Q-01). The design says "odyssey"; the name is
//! not decided, so a rename is a change here and in `ui/brand.js`.

/// Shown in the window title and the taskbar.
pub const NAME: &str = "odyssey";

/// Folder name for the profile. Uses the project's working name so it never collides with the
/// old Odyssey browser's data.
pub const DATA_DIR: &str = "Constellations";
