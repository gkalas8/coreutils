// This file is part of the uutils coreutils package.
//
// (c) Alan Andrade <alan.andradec@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// Synced with:
//  http://ftp-archive.freebsd.org/mirror/FreeBSD-Archive/old-releases/i386/1.0-RELEASE/ports/shellutils/src/id.c
//  http://www.opensource.apple.com/source/shell_cmds/shell_cmds-118/id/id.c
//
// This is not based on coreutils (8.32) GNU's `id`.
// This is based on BSD's `id` (noticeable in functionality, usage text, options text, etc.)
//
// Option '--zero' does not exist for BSD's `id`, therefor '--zero' is only allowed together
// with other options that are available on GNU's `id`.

// spell-checker:ignore (ToDO) asid auditid auditinfo auid cstr egid emod euid getaudit getlogin gflag nflag pline rflag termid uflag gsflag zflag

#![allow(non_camel_case_types)]
#![allow(dead_code)]

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use std::ffi::CStr;
use uucore::entries::{self, Group, Locate, Passwd};
pub use uucore::libc;
use uucore::libc::{getlogin, uid_t};
use uucore::process::{getegid, geteuid, getgid, getuid};

macro_rules! cstr2cow {
    ($v:expr) => {
        unsafe { CStr::from_ptr($v).to_string_lossy() }
    };
}

#[cfg(not(target_os = "linux"))]
mod audit {
    use super::libc::{c_int, c_uint, dev_t, pid_t, uid_t};

    pub type au_id_t = uid_t;
    pub type au_asid_t = pid_t;
    pub type au_event_t = c_uint;
    pub type au_emod_t = c_uint;
    pub type au_class_t = c_int;
    pub type au_flag_t = u64;

    #[repr(C)]
    pub struct au_mask {
        pub am_success: c_uint,
        pub am_failure: c_uint,
    }
    pub type au_mask_t = au_mask;

    #[repr(C)]
    pub struct au_tid_addr {
        pub port: dev_t,
    }
    pub type au_tid_addr_t = au_tid_addr;

    #[repr(C)]
    pub struct c_auditinfo_addr {
        pub ai_auid: au_id_t,         // Audit user ID
        pub ai_mask: au_mask_t,       // Audit masks.
        pub ai_termid: au_tid_addr_t, // Terminal ID.
        pub ai_asid: au_asid_t,       // Audit session ID.
        pub ai_flags: au_flag_t,      // Audit session flags
    }
    pub type c_auditinfo_addr_t = c_auditinfo_addr;

    extern "C" {
        pub fn getaudit(auditinfo_addr: *mut c_auditinfo_addr_t) -> c_int;
    }
}

static ABOUT: &str = "The id utility displays the user and group names and numeric IDs, of the calling process, to the standard output. If the real and effective IDs are different, both are displayed, otherwise only the real ID is displayed.\n\nIf a user (login name or user ID) is specified, the user and group IDs of that user are displayed. In this case, the real and effective IDs are assumed to be the same.";

mod options {
    pub const OPT_AUDIT: &str = "audit"; // GNU's id does not have this
    pub const OPT_EFFECTIVE_USER: &str = "user";
    pub const OPT_GROUP: &str = "group";
    pub const OPT_GROUPS: &str = "groups";
    pub const OPT_HUMAN_READABLE: &str = "human-readable"; // GNU's id does not have this
    pub const OPT_NAME: &str = "name";
    pub const OPT_PASSWORD: &str = "password"; // GNU's id does not have this
    pub const OPT_REAL_ID: &str = "real";
    pub const OPT_ZERO: &str = "zero"; // BSD's id does not have this
    pub const ARG_USERS: &str = "USER";
}

fn get_usage() -> String {
    format!("{0} [OPTION]... [USER]", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(crate_version!())
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(options::OPT_AUDIT)
                .short("A")
                .conflicts_with_all(&[options::OPT_GROUP, options::OPT_EFFECTIVE_USER, options::OPT_HUMAN_READABLE, options::OPT_PASSWORD, options::OPT_GROUPS, options::OPT_ZERO])
                .help("Display the process audit user ID and other process audit properties, which requires privilege (not available on Linux)."),
        )
        .arg(
            Arg::with_name(options::OPT_EFFECTIVE_USER)
                .short("u")
                .long(options::OPT_EFFECTIVE_USER)
                .conflicts_with(options::OPT_GROUP)
                .help("Display only the effective user ID as a number."),
        )
        .arg(
            Arg::with_name(options::OPT_GROUP)
                .short("g")
                .long(options::OPT_GROUP)
                .help("Display only the effective group ID as a number"),
        )
        .arg(
            Arg::with_name(options::OPT_GROUPS)
                .short("G")
                .long(options::OPT_GROUPS)
                .conflicts_with_all(&[options::OPT_GROUP, options::OPT_EFFECTIVE_USER, options::OPT_HUMAN_READABLE, options::OPT_PASSWORD, options::OPT_AUDIT])
                .help("Display only the different group IDs as white-space separated numbers, in no particular order."),
        )
        .arg(
            Arg::with_name(options::OPT_HUMAN_READABLE)
                .short("p")
                .help("Make the output human-readable. Each display is on a separate line."),
        )
        .arg(
            Arg::with_name(options::OPT_NAME)
                .short("n")
                .long(options::OPT_NAME)
                .help("Display the name of the user or group ID for the -G, -g and -u options instead of the number. If any of the ID numbers cannot be mapped into names, the number will be displayed as usual."),
        )
        .arg(
            Arg::with_name(options::OPT_PASSWORD)
                .short("P")
                .help("Display the id as a password file entry."),
        )
        .arg(
            Arg::with_name(options::OPT_REAL_ID)
                .short("r")
                .long(options::OPT_REAL_ID)
                .help("Display the real ID for the -g and -u options instead of the effective ID."),
        )
        .arg(
            Arg::with_name(options::OPT_ZERO)
                .short("z")
                .long(options::OPT_ZERO)
                .help("delimit entries with NUL characters, not whitespace;\nnot permitted in default format"),
        )
        .arg(
            Arg::with_name(options::ARG_USERS)
                .multiple(true)
                .takes_value(true)
                .value_name(options::ARG_USERS),
        )
        .get_matches_from(args);

    let nflag = matches.is_present(options::OPT_NAME);
    let uflag = matches.is_present(options::OPT_EFFECTIVE_USER);
    let gflag = matches.is_present(options::OPT_GROUP);
    let gsflag = matches.is_present(options::OPT_GROUPS);
    let rflag = matches.is_present(options::OPT_REAL_ID);
    let zflag = matches.is_present(options::OPT_ZERO);

    // "default format" is when none of '-ugG' was used
    // could not implement these "required" rules with just clap
    if (nflag || rflag) && !(uflag || gflag || gsflag) {
        crash!(1, "cannot print only names or real IDs in default format");
    }
    if (zflag) && !(uflag || gflag || gsflag) {
        crash!(1, "option --zero not permitted in default format");
    }

    let users: Vec<String> = matches
        .values_of(options::ARG_USERS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if matches.is_present(options::OPT_AUDIT) {
        auditid();
        return 0;
    }

    let possible_pw = if users.is_empty() {
        None
    } else {
        match Passwd::locate(users[0].as_str()) {
            Ok(p) => Some(p),
            Err(_) => crash!(1, "No such user/group: {}", users[0]),
        }
    };

    let line_ending = if zflag { '\0' } else { '\n' };

    if gflag {
        let id = possible_pw
            .map(|p| p.gid())
            .unwrap_or(if rflag { getgid() } else { getegid() });
        print!(
            "{}{}",
            if nflag {
                entries::gid2grp(id).unwrap_or_else(|_| id.to_string())
            } else {
                id.to_string()
            },
            line_ending
        );
        return 0;
    }

    if uflag {
        let id = possible_pw
            .map(|p| p.uid())
            .unwrap_or(if rflag { getuid() } else { geteuid() });
        print!(
            "{}{}",
            if nflag {
                entries::uid2usr(id).unwrap_or_else(|_| id.to_string())
            } else {
                id.to_string()
            },
            line_ending
        );
        return 0;
    }

    if gsflag {
        let delimiter = if zflag { "" } else { " " };
        print!(
            "{}{}",
            if nflag {
                possible_pw
                    .map(|p| p.belongs_to())
                    .unwrap_or_else(|| entries::get_groups().unwrap())
                    .iter()
                    .map(|&id| entries::gid2grp(id).unwrap())
                    .collect::<Vec<_>>()
                    .join(delimiter)
            } else {
                possible_pw
                    .map(|p| p.belongs_to())
                    .unwrap_or_else(|| entries::get_groups().unwrap())
                    .iter()
                    .map(|&id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(delimiter)
            },
            line_ending
        );
        return 0;
    }

    if matches.is_present(options::OPT_PASSWORD) {
        pline(possible_pw.map(|v| v.uid()));
        return 0;
    };

    if matches.is_present(options::OPT_HUMAN_READABLE) {
        pretty(possible_pw);
        return 0;
    }

    if possible_pw.is_some() {
        id_print(possible_pw, false, false)
    } else {
        id_print(possible_pw, true, true)
    }

    0
}

fn pretty(possible_pw: Option<Passwd>) {
    if let Some(p) = possible_pw {
        print!("uid\t{}\ngroups\t", p.name());
        println!(
            "{}",
            p.belongs_to()
                .iter()
                .map(|&gr| entries::gid2grp(gr).unwrap())
                .collect::<Vec<_>>()
                .join(" ")
        );
    } else {
        let login = cstr2cow!(getlogin() as *const _);
        let rid = getuid();
        if let Ok(p) = Passwd::locate(rid) {
            if login == p.name() {
                println!("login\t{}", login);
            }
            println!("uid\t{}", p.name());
        } else {
            println!("uid\t{}", rid);
        }

        let eid = getegid();
        if eid == rid {
            if let Ok(p) = Passwd::locate(eid) {
                println!("euid\t{}", p.name());
            } else {
                println!("euid\t{}", eid);
            }
        }

        let rid = getgid();
        if rid != eid {
            if let Ok(g) = Group::locate(rid) {
                println!("euid\t{}", g.name());
            } else {
                println!("euid\t{}", rid);
            }
        }

        println!(
            "groups\t{}",
            entries::get_groups()
                .unwrap()
                .iter()
                .map(|&gr| entries::gid2grp(gr).unwrap())
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
}

#[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
fn pline(possible_uid: Option<uid_t>) {
    let uid = possible_uid.unwrap_or_else(getuid);
    let pw = Passwd::locate(uid).unwrap();

    println!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        pw.name(),
        pw.user_passwd(),
        pw.uid(),
        pw.gid(),
        pw.user_access_class(),
        pw.passwd_change_time(),
        pw.expiration(),
        pw.user_info(),
        pw.user_dir(),
        pw.user_shell()
    );
}

#[cfg(target_os = "linux")]
fn pline(possible_uid: Option<uid_t>) {
    let uid = possible_uid.unwrap_or_else(getuid);
    let pw = Passwd::locate(uid).unwrap();

    println!(
        "{}:{}:{}:{}:{}:{}:{}",
        pw.name(),
        pw.user_passwd(),
        pw.uid(),
        pw.gid(),
        pw.user_info(),
        pw.user_dir(),
        pw.user_shell()
    );
}

#[cfg(target_os = "linux")]
fn auditid() {}

#[cfg(not(target_os = "linux"))]
fn auditid() {
    #[allow(deprecated)]
    let mut auditinfo: audit::c_auditinfo_addr_t = unsafe { std::mem::uninitialized() };
    let address = &mut auditinfo as *mut audit::c_auditinfo_addr_t;
    if unsafe { audit::getaudit(address) } < 0 {
        println!("couldn't retrieve information");
        return;
    }

    println!("auid={}", auditinfo.ai_auid);
    println!("mask.success=0x{:x}", auditinfo.ai_mask.am_success);
    println!("mask.failure=0x{:x}", auditinfo.ai_mask.am_failure);
    println!("termid.port=0x{:x}", auditinfo.ai_termid.port);
    println!("asid={}", auditinfo.ai_asid);
}

fn id_print(possible_pw: Option<Passwd>, p_euid: bool, p_egid: bool) {
    let (uid, gid) = possible_pw
        .map(|p| (p.uid(), p.gid()))
        .unwrap_or((getuid(), getgid()));

    let groups = match Passwd::locate(uid) {
        Ok(p) => p.belongs_to(),
        Err(e) => crash!(1, "Could not find uid {}: {}", uid, e),
    };

    print!("uid={}({})", uid, entries::uid2usr(uid).unwrap());
    print!(" gid={}({})", gid, entries::gid2grp(gid).unwrap());

    let euid = geteuid();
    if p_euid && (euid != uid) {
        print!(" euid={}({})", euid, entries::uid2usr(euid).unwrap());
    }

    let egid = getegid();
    if p_egid && (egid != gid) {
        print!(" egid={}({})", euid, entries::gid2grp(egid).unwrap());
    }

    println!(
        " groups={}",
        groups
            .iter()
            .map(|&gr| format!("{}({})", gr, entries::gid2grp(gr).unwrap()))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn get_groups() ->
