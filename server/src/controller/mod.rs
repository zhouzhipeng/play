mod index_controller;
mod static_controller;
mod user_controller;
mod ws_controller;
pub mod function_controller;
mod todo_item_controller;
mod api_entry_controller;
mod admin_controller;
mod english_card_controller;
mod email_inbox_controller;
mod finance_controller;
mod shortlink_controller;
mod job_controller;


mod general_data_controller;
mod pages_controller;
mod shell_controller;
mod files_controller;
pub mod chat_controller;
//PLACEHOLDER:CONTROLLER_MOD



///
/// register your controllers here.
crate::register_routers!(
        index_controller,
        static_controller,
        user_controller,
        ws_controller,
        function_controller,
        todo_item_controller,
        api_entry_controller,
        admin_controller,
        english_card_controller,
        email_inbox_controller,
        finance_controller,
        shortlink_controller,
        job_controller,
        general_data_controller,
        pages_controller,
        shell_controller,
        files_controller,
        chat_controller,
        //PLACEHOLDER:CONTROLLER_REGISTER


    );

