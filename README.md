Memoize
=======

This is a simple tool for sharing notes.
You have a git repository full of Markdown files; Memoize renders them for convenient reading on the web.
It's the world's most basic knowledge base.

Why?
----

This is not an original observation, but in my research lab, we've found it's a really good idea to have a central repository of written notes that we all share.
Some examples of the kinds of notes we share are research proposals, little guides on how to install something, or "living document" SOPs about how the lab works.
In the real world, I think people do stuff like this using Google Docs, a wiki, a "knowledge base" like [Notion][] or [Outline][], or an RFC-like repository like [Oxide's][oxide-rfd].
But we just have a git repository full of Markdown files, which is much more "us."

If you have one of these git repositories full of Markdown notes, you can view them conveniently in the GitHub web interface.
Honestly, that's a pretty good way to do it; the reasonable thing to do would be to just stick with that.
But wouldn't it be even nicer if the rendered versions were just nice, clean web pages instead of having all that GitHub cruft?

Memoize is essentially a static site generator specialized for rendering these notes.
It converts your directory of Markdown files into a directory of HTML files ready for uploading somewhere.

[notion]: https://www.notion.com
[outline]: https://www.getoutline.com
[oxide-rfd]: https://rfd.shared.oxide.computer
[pandoc]: https://pandoc.org

Features
--------

For our lab's notes, Memoize replaced [Pandoc][] and some Makefile jiggery-pokery.
Again, the sensible thing to do would have been just to stick with that; Memoize is a fun exercise in overengineering that comes with a few more features:

* A nice, clean built-in template and responsive stylesheet with a dark mode.
* A sticky table of contents for navigating the headings in each note.
* A "serve" mode with live reloading for previewing while editing.
* Pages that display metadata from git: the last modified date, the last author, and that sort of thing. Also a link to GitHub for in-browser editing, if you want that.
* Parallel builds.
* Relative links between Markdown files work: e.g., a link to `./foo.md` in the Markdown becomes a link to `./foo.html` in the rendered site.

Render Your Notes
-----------------

Go to your directory with your Markdown notes and type `memoize build`.
You'll now have a `_public` directory with all your rendered notes.

Here are some things to know about the generated site:

* Any Markdown file named `*.md` gets converted into an equivalent, self-contained `*.html`.
* Non-Markdown files (e.g., images) get copied as-is. (Actually, we use hard links when we can.)
* The generated site mirrors the subdirectory structure of the source directory, so go ahead and organize notes into a hierarchy if you like.
* Filenames that start with `.` and `_` are excluded.

Preview Server
--------------

While writing notes, type `memoize serve` to start a server.
Memoize will watch your source directory for changes and refresh the page for you.

Credits
-------

Memoize is by [Adrian Sampson][adrian].
It was created for the [Capra][] lab.
The license is [MIT][].

[adrian]: https://www.cs.cornell.edu/~asampson/
[capra]: https://capra.cs.cornell.edu
[mit]: https://choosealicense.com/licenses/mit/
