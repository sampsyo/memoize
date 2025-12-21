pub mod assets;
pub mod core;
pub mod markdown;

use argh::FromArgs;
use core::Context;
use std::io;

#[derive(FromArgs)]
/// a static knowledge base
struct Knot2 {
    #[argh(subcommand)]
    mode: Command,

    #[argh(option, default = "String::from(\".\")")]
    /// source directory
    source: String,

    #[argh(option, default = "String::from(\"_public\")")]
    /// destination directory
    dest: String,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
    Build(BuildCommand),
    Show(ShowCommand),
}

#[derive(FromArgs)]
/// build the full site
#[argh(subcommand, name = "build")]
struct BuildCommand {}

#[derive(FromArgs)]
/// print a single file from the site
#[argh(subcommand, name = "show")]
struct ShowCommand {
    #[argh(positional)]
    /// a relative path to the file to render
    path: String,
}

fn main() {
    let args: Knot2 = argh::from_env();
    let ctx = Context::new(&args.source, &args.dest);
    match args.mode {
        Command::Build(_) => ctx.render_site().unwrap(),
        Command::Show(cmd) => match ctx.resolve_resource(&cmd.path) {
            Some(rsrc) => {
                ctx.render_resource(rsrc, &mut io::stdout()).unwrap();
            }
            None => println!("not found"),
        },
    }
}
