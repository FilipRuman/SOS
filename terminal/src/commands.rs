use core::str::FromStr;

use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use log::*;

use crate::Terminal;
fn get_first_arg<T: FromStr + Default>(args: Vec<&str>) -> T {
    match args.get(0) {
        Some(val) => match val.parse::<T>() {
            Ok(parsed) => parsed,
            Err(_) => {
                warn!("failed to parse value to target type, setting default value");
                T::default()
            }
        },
        None => {
            warn!("value was not specified! setting default value");
            T::default()
        }
    }
}

pub type OnCommandFunction = fn(&mut Terminal, Vec<&str>);
fn set_log_level(terminal: &mut Terminal, args: Vec<&str>) {
    terminal.logs = get_first_arg(args);
    debug!("logs are set to: {}", terminal.logs)
}
pub fn init_commands() -> BTreeMap<String, OnCommandFunction> {
    BTreeMap::from([
        (
            "poweroff".to_string(),
            (|_, _| os::shutdown()) as OnCommandFunction,
        ),
        ("logs".to_string(), set_log_level as OnCommandFunction),
    ])
}
impl Terminal {
    pub fn parse_and_run_current_command(&mut self) {
        let current_str_input = self.current_str_input.clone();
        let mut split = current_str_input.split(" ");
        let command_name = match split.next() {
            Some(str) => str,
            None => {
                warn!("you need to specify command name!");
                return;
            }
        };

        let args: Vec<&str> = split.collect();
        match self.commands.get(command_name) {
            Some(command_function) => {
                command_function(self, args);
            }
            None => {
                warn!(
                    "no command with name: { } was found, all commands: {:?}",
                    command_name,
                    self.commands.keys()
                );
            }
        };
    }
}
