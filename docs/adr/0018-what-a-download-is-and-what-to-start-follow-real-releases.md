# 0018 What a download is, and what to start, follow the releases the catalog really points at

- status: accepted
- date: 2026-10-06
- spec: ../specs/install.md

## Context

The installer had only been run against a fake GitHub serving archives the tests built. Run against the community catalog's real
releases (`reclaw-install/tests/live.rs`, `reclaw-app/src/host/install/tests/live.rs`), three entries could not be installed or were
misjudged:

* GitLab package links carry the release's version in their name and no extension: `MarioKart64Recompiled-v0.9.2-Linux-X64-Release`,
  `LADXHD.Patcher.Lite-Linux-x64`. The text after the last dot (`2-linux-x64-release`) was taken for an extension nobody knows, so
  every sonicdcer recompilation (Mario Kart 64, Star Fox 64, Duke Nukem: Zero Hour, Extreme-G) and Link's Awakening DX HD was refused
  with "a file of a kind Reclaw does not install", before the download that would have said what it is.
* doukutsu-rs ships its Linux build as one program named `doukutsu-rs_linux_1.0.0.x86_64.elf`; `.elf` was refused the same way.
* Digimon World Recompiled ships a Windows zip with its sources in it, and among them a library's test fixtures that are x86-64 ELF
  programs, six folders down. Any ELF file made the app "native", so Reclaw reported one of those fixtures as the program to start and
  did not say the game needs Wine or Proton.

## Decision

* **A name has an extension only if what follows its last dot is one to five letters or digits.** Anything else is a name with no
  extension, and the downloaded file's first bytes decide (zip, 7z, rar, gzip, xz; otherwise a program in one file). `.elf`, `.x86_64`,
  `.arm64` and `.aarch64` are names for a program in one file.
* **On Linux, a Windows `.exe` makes the app a Windows app when it is nearer the top of the folder than every native program.** A
  native program as near the top as the `.exe` still wins, so a Linux build that carries a Windows helper beside its program stays native.

## Rejected

* **Using GitLab's `link_type: package`, or the `Content-Disposition` file name, to name the file.** The first is only GitLab's and says
  nothing about the kind; the second is known only after the request, while the choice of file (which uses the name) is made before.
  The first bytes already decide for a name with no extension; the fix is to recognise that such names have none.
* **Requiring a native program to be built for this machine's architecture (the ELF `e_machine`).** It removes the PowerPC and ARM
  fixtures but not the x86-64 ones (`asm64`, `hello_64`, `ls` ...) in the same Windows zip, so it would not have fixed the case that
  was found. It may still be worth having for releases that bundle several architectures; not done.
* **Searching only the top of the folder.** Many Linux builds keep their program in `bin/`; the nearest-first order already prefers the
  top without missing those.
