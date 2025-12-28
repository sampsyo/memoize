use crate::core::Context;
use std::path::Path;
use std::process::Command;

pub struct CommitData(Vec<u8>);

#[derive(Debug)]
pub struct CommitInfo<'a> {
    pub hash: &'a str,
    pub date: &'a str,
    pub email: &'a str,
    pub name: &'a str,
}

impl CommitData {
    pub fn info<'a>(&'a self) -> CommitInfo<'a> {
        let res = str::from_utf8(&self.0)
            .expect("git output must be UTF-8")
            .strip_suffix("\n")
            .expect("missing newline at end of output");
        let mut parts = res.splitn(4, " ");
        CommitInfo {
            hash: parts.next().unwrap(),
            date: parts.next().unwrap(),
            email: parts.next().unwrap(),
            name: parts.next().unwrap(),
        }
    }
}

pub fn last_commit(repo: &Path, file: &Path) -> std::io::Result<CommitData> {
    let stdout = Command::new("git")
        .current_dir(repo)
        .args([
            "log",
            "-1",
            "--format=%H %cs %ce %cn",
            "--",
            file.to_str().expect("path must be UTF-8"),
        ])
        .output()?
        .stdout;
    // TODO check exit status?
    Ok(CommitData(stdout))
}

pub fn blarg(ctx: Context) {
    let commit = last_commit(&ctx.src_dir, Path::new("Cargo.toml")).unwrap();
    dbg!(commit.info());
}
