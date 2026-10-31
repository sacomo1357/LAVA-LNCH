mod auth;
mod instances;
mod java;
mod mc;
mod multi;
mod settings;
mod skin;
mod update;
mod util;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            settings::get_settings,
            settings::save_settings,
            settings::open_folder,
            settings::open_url,
            settings::builtin_client_id,
            java::java_detect,
            java::java_install,
            auth::auth_offline,
            auth::auth_logout,
            auth::auth_ms_login,
            auth::auth_ms_cancel,
            mc::mc_versions,
            instances::list_instances,
            instances::create_instance,
            instances::delete_instance,
            instances::launch_instance,
            instances::list_mods,
            instances::toggle_mod,
            instances::delete_mod,
            instances::modrinth_search,
            instances::modrinth_install,
            settings::bg_get,
            settings::bg_set,
            multi::server_add,
            multi::multi_play,
            skin::skin_get,
            skin::skin_list,
            skin::skin_add,
            skin::skin_delete,
            skin::skin_apply,
            update::update_check,
            update::update_install,
            update::update_default_repo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running LAVA SE Launcher");
}
