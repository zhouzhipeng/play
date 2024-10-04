mod index_controller;
pub mod static_controller;
mod ws_controller;
pub mod function_controller;
mod admin_controller;
pub mod shortlink_controller;
mod job_controller;


mod general_data_controller;
mod pages_controller;
mod shell_controller;
mod files_controller;
pub mod chat_controller;
pub mod cache_controller;
mod test_controller;
pub mod plugin_controller;
//PLACEHOLDER:CONTROLLER_MOD




///
/// register your controllers here.
crate::register_routers!(
        index_controller,
        static_controller,
        ws_controller,
        function_controller,
        admin_controller,
        job_controller,
        general_data_controller,
        pages_controller,
        shell_controller,
        files_controller,
        chat_controller,
        cache_controller,
        test_controller,
        plugin_controller,
//PLACEHOLDER:CONTROLLER_REGISTER



    );

