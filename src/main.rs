pub mod assets;
pub mod core;
pub mod markdown;
pub mod serve;

use argh::FromArgs;
use core::Context;
use std::io;
use std::path::Path;

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
    List(ListCommand),
    Serve(ServeCommand),
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

#[derive(FromArgs)]
/// list the resources in a site
#[argh(subcommand, name = "list")]
struct ListCommand {}

#[derive(FromArgs)]
/// run a web server
#[argh(subcommand, name = "serve")]
struct ServeCommand {}

fn main() {
    let args: Knot2 = argh::from_env();
    let ctx = Context::new(&args.source);
    match args.mode {
        Command::Build(_) => {
            let dest_path = Path::new(&args.dest);
            ctx.render_site(dest_path).unwrap()
        }
        Command::Show(cmd) => match ctx.resolve_resource(&cmd.path) {
            Some(rsrc) => {
                ctx.render_resource(rsrc, &mut io::stdout()).unwrap();
            }
            None => eprintln!("not found"),
        },
        Command::List(_) => {
            for rsrc in ctx.read_resources() {
                match rsrc {
                    core::Resource::Directory(path) => println!("dir  {}", path.display()),
                    core::Resource::Static(path) => println!("file {}", path.display()),
                    core::Resource::Note(path) => println!("note {}", path.display()),
                }
            }
        }
        Command::Serve(_) => {
            serve::serve(ctx);
        }
    }
}
