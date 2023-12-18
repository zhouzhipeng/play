mod index_controller;
mod static_controller;
mod user_controller;
mod ws_controller;
mod function_controller;
mod todo_controller;
mod api_entry_controller;
mod admin_controller;


///
/// register your controllers here.
crate::register_routers!(
        index_controller,
        static_controller,
        user_controller,
        ws_controller,
        function_controller,
        todo_controller,
        api_entry_controller,
        admin_controller,
    );

