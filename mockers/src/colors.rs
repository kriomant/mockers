
// Copied from 'colorify' package because it has no plain 'bold' variant.
#[macro_export]
macro_rules! colored {
    (bold: $s:expr) => {
        concat!("\x1b[1m", $s, "\x1b[0m")
    };
    (red: $s:expr) => {
        concat!("\x1b[31m", $s, "\x1b[0m")
    };
    (red_bold: $s:expr) => {
        concat!("\x1b[1;31m", $s, "\x1b[0m")
    };
    (green: $s:expr) => {
        concat!("\x1b[32m", $s, "\x1b[0m")
    };
    (green_bold: $s:expr) => {
        concat!("\x1b[1;32m", $s, "\x1b[0m")
    };
    (orange: $s:expr) => {
        concat!("\x1b[33m", $s, "\x1b[0m")
    };
    (yellow_bold: $s:expr) => {
        concat!("\x1b[1;33m", $s, "\x1b[0m")
    };
    (blue: $s:expr) => {
        concat!("\x1b[34m", $s, "\x1b[0m")
    };
    (blue_bold: $s:expr) => {
        concat!("\x1b[1;34m", $s, "\x1b[0m")
    };
    (purple: $s:expr) => {
        concat!("\x1b[35m", $s, "\x1b[0m")
    };
    (purple_bold: $s:expr) => {
        concat!("\x1b[1;35m", $s, "\x1b[0m")
    };
    (cyan: $s:expr) => {
        concat!("\x1b[36m", $s, "\x1b[0m")
    };
    (cyan_bold: $s:expr) => {
        concat!("\x1b[1;36m", $s, "\x1b[0m")
    };
    (light_grey: $s:expr) => {
        concat!("\x1b[37m", $s, "\x1b[0m")
    };
    (white_bold: $s:expr) => {
        concat!("\x1b[1;37m", $s, "\x1b[0m")
    };
    (dark_grey: $s:expr) => {
        concat!("\x1b[90m", $s, "\x1b[0m")
    };
    (dark_grey_bold: $s:expr) => {
        concat!("\x1b[1;90m", $s, "\x1b[0m")
    };
    (peach: $s:expr) => {
        concat!("\x1b[91m", $s, "\x1b[0m")
    };
    (peach_bold: $s:expr) => {
        concat!("\x1b[1;91m", $s, "\x1b[0m")
    };
    (lime: $s:expr) => {
        concat!("\x1b[92m", $s, "\x1b[0m")
    };
    (lime_bold: $s:expr) => {
        concat!("\x1b[1;92m", $s, "\x1b[0m")
    };
    (yellow: $s:expr) => {
        concat!("\x1b[93m", $s, "\x1b[0m")
    };
    (yellow_bold: $s:expr) => {
        concat!("\x1b[1;93m", $s, "\x1b[0m")
    };
    (royal_blue: $s:expr) => {
        concat!("\x1b[94m", $s, "\x1b[0m")
    };
    (royal_blue_bold: $s:expr) => {
        concat!("\x1b[1;94m", $s, "\x1b[0m")
    };
    (magenta: $s:expr) => {
        concat!("\x1b[95m", $s, "\x1b[0m")
    };
    (magenta_bold: $s:expr) => {
        concat!("\x1b[1;95m", $s, "\x1b[0m")
    };
    (teal: $s:expr) => {
        concat!("\x1b[96m", $s, "\x1b[0m")
    };
    (teal_bold: $s:expr) => {
        concat!("\x1b[1;96m", $s, "\x1b[0m")
    };
    (white: $s:expr) => {
        concat!("\x1b[97m", $s, "\x1b[0m")
    };
    (white_bold: $s:expr) => {
        concat!("\x1b[1;97m", $s, "\x1b[0m")
    };
}
