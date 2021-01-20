pub fn is_debug() -> bool {
    true
}

pub fn get_swayr_socket_path() -> String {
    format!("/run/user/{}/swayr-sock", users::get_current_uid())
}
