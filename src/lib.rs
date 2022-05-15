// #![allow(
//     dead_code,
//     unused_imports,
//     unused_variables,
//     clippy::missing_safety_doc
// )]
#![allow(clippy::missing_safety_doc)]

use crate::ffi::loadable_extension_init;
use crate::ffi::sqlite3_auto_extension;
use anyhow::Context as ACtxt;
use log::LevelFilter;
use regex::bytes::Regex;
use rusqlite::ffi;
use rusqlite::functions::{Context, FunctionFlags};
use rusqlite::types::{ToSqlOutput, Value, ValueRef};
use rusqlite::Connection;
use std::os::raw::c_int;

fn ah(e: anyhow::Error) -> rusqlite::Error {
    rusqlite::Error::UserFunctionError(format!("{:?}", e).into())
}

fn init_logging(default_level: LevelFilter) {
    let lib_log_env = "SQLITE_REGEX_LOG";
    if std::env::var(lib_log_env).is_err() {
        std::env::set_var(lib_log_env, format!("{}", default_level))
    }

    let logger_env = env_logger::Env::new().filter(lib_log_env);

    env_logger::try_init_from_env(logger_env).ok();
}

// Will use with     ffi:sqlite3_auto_extension(arg1)
// https://www.sqlite.org/c3ref/auto_extension.html
// Example: https://sqlite.org/src/file/ext/misc/vfsstat.c
// https://www.sqlite.org/loadext.html
// #[no_mangle]
// pub unsafe extern "C" fn regex_register(
//     db: *mut ffi::sqlite3,
//     _pz_err_msg: &mut &mut std::os::raw::c_char,
//     p_api: *mut ffi::sqlite3_api_routines,
// ) -> c_int {}

#[no_mangle]
pub unsafe extern "C" fn sqlite3_regex_init_internal(
    db: *mut ffi::sqlite3,
    _pz_err_msg: &mut &mut std::os::raw::c_char,
    p_api: *mut ffi::sqlite3_api_routines,
) -> c_int {
    // https://www.sqlite.org/loadext.html
    // https://github.com/jgallagher/rusqlite/issues/524#issuecomment-507787350
    // SQLITE_EXTENSION_INIT2 equivalent
    loadable_extension_init(p_api);
    /* Insert here calls to
     **     sqlite3_create_function_v2(),
     **     sqlite3_create_collation_v2(),
     **     sqlite3_create_module_v2(), and/or
     **     sqlite3_vfs_register()
     ** to register the new features that your extension adds.
     */
    match init(db) {
        Ok(()) => {
            log::info!("[regex-extension] init ok");
            // ffi::SQLITE_OK
            ffi::SQLITE_OK_LOAD_PERMANENTLY
        }

        Err(e) => {
            log::error!("[regex-extension] init error: {:?}", e);
            ffi::SQLITE_ERROR
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn sqlite3_regex_init(
    db: *mut ffi::sqlite3,
    _pz_err_msg: &mut &mut std::os::raw::c_char,
    p_api: *mut ffi::sqlite3_api_routines,
) -> c_int {
    loadable_extension_init(p_api);
    let ptr = sqlite3_regex_init_internal
        as unsafe extern "C" fn(
            *mut ffi::sqlite3,
            &mut &mut std::os::raw::c_char,
            *mut ffi::sqlite3_api_routines,
        ) -> c_int;

    sqlite3_auto_extension(Some(std::mem::transmute(ptr)));
    match init(db) {
        Ok(()) => {
            log::info!("[regex-extension] init ok");
            ffi::SQLITE_OK_LOAD_PERMANENTLY
        }

        Err(e) => {
            log::error!("[regex-extension] init error: {:?}", e);
            ffi::SQLITE_ERROR
        }
    }
}

fn init(db_handle: *mut ffi::sqlite3) -> anyhow::Result<()> {
    let db = unsafe { rusqlite::Connection::from_handle(db_handle)? };
    load(&db)?;
    Ok(())
}

fn load(c: &Connection) -> anyhow::Result<()> {
    load_with_loglevel(c, LevelFilter::Info)
}

fn load_with_loglevel(c: &Connection, default_log_level: LevelFilter) -> anyhow::Result<()> {
    init_logging(default_log_level);
    add_functions(c)
}

fn add_functions(c: &Connection) -> anyhow::Result<()> {
    let deterministic = FunctionFlags::SQLITE_DETERMINISTIC | FunctionFlags::SQLITE_UTF8;
    // | FunctionFlags::SQLITE_INNOCUOUS;

    c.create_scalar_function("regex_extract", 2, deterministic, |ctx: &Context| {
        regex_extract(ctx).map_err(ah)
    })?;

    c.create_scalar_function("regex_extract", 3, deterministic, |ctx: &Context| {
        regex_extract(ctx).map_err(ah)
    })?;

    Ok(())
}

fn regex_extract<'a>(ctx: &Context) -> anyhow::Result<ToSqlOutput<'a>> {
    let arg_pat = 0;
    let arg_input_data = 1;
    let arg_cap_group = 2;

    let empty_return = Ok(ToSqlOutput::Owned(Value::Null));

    let pattern = match ctx.get_raw(arg_pat) {
        ValueRef::Text(t) => t,
        e => anyhow::bail!("regex pattern must be text, got {}", e.data_type()),
    };

    let re = Regex::new(std::str::from_utf8(pattern)?)?;

    let input_value = match ctx.get_raw(arg_input_data) {
        ValueRef::Text(t) => t,
        ValueRef::Null => return empty_return,
        e => anyhow::bail!("regex expects text as input, got {}", e.data_type()),
    };

    let cap_group: usize = if ctx.len() <= arg_cap_group {
        // no capture group, use default
        0
    } else {
        ctx.get(arg_cap_group).context("capture group")?
    };

    // let mut caploc = re.capture_locations();
    // re.captures_read(&mut caploc, input_value);
    if let Some(cap) = re.captures(input_value) {
        match cap.get(cap_group) {
            None => empty_return,
            // String::from_utf8_lossy
            Some(t) => {
                let value = String::from_utf8_lossy(t.as_bytes());
                return Ok(ToSqlOutput::Owned(Value::Text(value.to_string())));
            }
        }
    } else {
        empty_return
    }
}
