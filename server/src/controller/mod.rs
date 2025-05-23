mod index_controller;
pub mod static_controller;
mod ws_controller;
pub mod function_controller;
mod admin_controller;
pub mod shortlink_controller;
mod job_controller;


mod data_v1_controller;
pub mod pages_controller;
mod shell_controller;
pub mod files_controller;
pub mod cache_controller;
mod test_controller;
pub mod plugin_controller;
mod data_v3_controller;
mod data_v2_controller;
mod data_v4_controller;
pub mod redis_controller;
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
        data_v1_controller,
        pages_controller,
        shell_controller,
        files_controller,
        cache_controller,
        test_controller,
        data_v2_controller,
        data_v3_controller,
        data_v4_controller,
        redis_controller,
//PLACEHOLDER:CONTROLLER_REGISTER



    );

