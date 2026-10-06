use std::fs;

use super::*;

fn touch(path: &Path, executable: bool) {
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, "#!/bin/sh\n").expect("file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(if executable { 0o755 } else { 0o644 })).expect("mode");
    }
    let _ = executable;
}

struct Machine {
    _dir: tempfile::TempDir,
    probe: Probe,
    game: PathBuf,
    exe: PathBuf,
}

fn machine() -> Machine {
    let dir = tempfile::tempdir().expect("dir");
    let home = dir.path().join("home");
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).expect("bin");
    let game = dir.path().join("games/One");
    fs::create_dir_all(&game).expect("game");
    let exe = game.join("Game.exe");
    fs::write(&exe, "MZ").expect("exe");
    Machine { probe: Probe { path: vec![bin], home }, game, exe, _dir: dir }
}

fn steam_proton(m: &Machine, name: &str) -> PathBuf {
    let proton = m.probe.home.join(".local/share/Steam/steamapps/common").join(name).join("proton");
    touch(&proton, true);
    proton
}

#[test]
fn the_kind_is_read_from_the_library_word() {
    assert_eq!(RunnerKind::parse(Some(" Proton ")), RunnerKind::Proton);
    assert_eq!(RunnerKind::parse(Some("WINE")), RunnerKind::Wine);
    assert_eq!(RunnerKind::parse(Some("custom")), RunnerKind::Custom);
    assert_eq!(RunnerKind::parse(Some("whatever")), RunnerKind::Auto);
    assert_eq!(RunnerKind::parse(None), RunnerKind::Auto);
}

#[test]
fn numbers_in_names_are_compared_as_numbers() {
    let mut names = ["Proton 9.0", "Proton 10.0", "Proton 8.0-5", "Proton 10.0-1"];
    names.sort_by(|a, b| natural(b, a));
    assert_eq!(names, ["Proton 10.0-1", "Proton 10.0", "Proton 9.0", "Proton 8.0-5"]);
}

#[test]
fn the_newest_proton_comes_first_and_both_places_are_searched() {
    let m = machine();
    steam_proton(&m, "Proton 9.0");
    steam_proton(&m, "Proton 10.0");
    let custom = m.probe.home.join(".local/share/Steam/compatibilitytools.d/GE-Proton9-27/proton");
    touch(&custom, true);
    let installs = proton_installs(&m.probe);
    let names: Vec<_> = installs.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["Proton 10.0", "Proton 9.0", "GE-Proton9-27"]);
    assert!(installs.iter().all(|i| i.steam_root == m.probe.home.join(".local/share/Steam")));
}

#[test]
fn experimental_is_the_last_resort_not_the_first_choice() {
    let m = machine();
    steam_proton(&m, "Proton - Experimental");
    steam_proton(&m, "Proton 9.0");
    let names: Vec<_> = proton_installs(&m.probe).into_iter().map(|i| i.name).collect();
    assert_eq!(names, ["Proton 9.0", "Proton - Experimental"]);
}

#[test]
fn a_folder_without_a_proton_script_is_not_an_install() {
    let m = machine();
    fs::create_dir_all(m.probe.home.join(".local/share/Steam/steamapps/common/Proton Hotfix")).expect("dir");
    assert!(proton_installs(&m.probe).is_empty());
}

#[test]
fn auto_prefers_proton_to_wine_and_builds_steams_variables() {
    let m = machine();
    let proton = steam_proton(&m, "Proton 9.0");
    touch(&m.probe.path[0].join("wine"), true);
    let command = command(&RunnerConfig::default(), &m.probe, &m.exe, &m.game).expect("a command");
    assert_eq!(command.program, proton);
    assert_eq!(command.args, [OsString::from("waitforexitandrun"), m.exe.clone().into()]);
    let env: std::collections::HashMap<_, _> =
        command.env.iter().map(|(k, v)| (k.to_string_lossy().into_owned(), v.to_string_lossy().into_owned())).collect();
    assert_eq!(env["STEAM_COMPAT_DATA_PATH"], m.game.join(".steam-compat-data").to_string_lossy());
    assert_eq!(env["STEAM_COMPAT_CLIENT_INSTALL_PATH"], m.probe.home.join(".local/share/Steam").to_string_lossy());
    assert_eq!(env["SteamAppId"], env["STEAM_COMPAT_APP_ID"]);
    assert_eq!(env["STEAM_COMPAT_INSTALL_PATH"], m.game.to_string_lossy());
    assert!(m.game.join(".steam-compat-data").is_dir(), "the prefix folder is made");
    // The same program always gets the same id (and so the same prefix).
    assert_eq!(compat_app_id(&m.exe), env["STEAM_COMPAT_APP_ID"]);
    assert_ne!(compat_app_id(Path::new("/other/Game.exe")), compat_app_id(&m.exe));
}

#[test]
fn auto_falls_back_to_wine_with_its_own_prefix() {
    let m = machine();
    touch(&m.probe.path[0].join("wine"), true);
    let command = command(&RunnerConfig::default(), &m.probe, &m.exe, &m.game).expect("a command");
    assert_eq!(command.program, m.probe.path[0].join("wine"));
    assert_eq!(command.args, [OsString::from(&m.exe)]);
    assert_eq!(command.env, vec![(OsString::from("WINEPREFIX"), m.game.join(".wine-prefix").into_os_string())]);
    touch(&m.probe.path[0].join("wine64"), true);
    assert_eq!(
        super::command(&RunnerConfig::default(), &m.probe, &m.exe, &m.game).expect("a command").program,
        m.probe.path[0].join("wine64"),
        "wine64 first"
    );
}

#[test]
fn a_wine_that_is_not_executable_is_not_wine() {
    let m = machine();
    touch(&m.probe.path[0].join("wine"), false);
    assert_eq!(command(&RunnerConfig::default(), &m.probe, &m.exe, &m.game), Err(RunnerError::None));
    assert!(!available(&RunnerConfig::default(), &m.probe));
}

#[test]
fn a_global_command_beats_everything_in_auto() {
    let m = machine();
    steam_proton(&m, "Proton 9.0");
    let config = RunnerConfig { global_custom: Some("umu-run {exe}".into()), ..RunnerConfig::default() };
    let command = command(&config, &m.probe, &m.exe, &m.game).expect("a command");
    assert_eq!((command.program, command.args), (PathBuf::from("umu-run"), vec![OsString::from(&m.exe)]));
}

#[test]
fn a_custom_command_fills_its_placeholders_and_adds_the_program_when_it_names_none() {
    let m = machine();
    let config = RunnerConfig {
        kind: RunnerKind::Custom,
        custom: Some("env FOO=1 'my wine' --dir {exeDir} --game {gamePath}".into()),
        ..RunnerConfig::default()
    };
    let command = command(&config, &m.probe, &m.exe, &m.game).expect("a command");
    assert_eq!(command.program, PathBuf::from("env"));
    assert_eq!(
        command.args,
        ["FOO=1", "my wine", "--dir", &m.game.to_string_lossy(), "--game", &m.game.to_string_lossy()].map(OsString::from),
    );
    let bare = RunnerConfig { kind: RunnerKind::Custom, custom: Some("wine --fast".into()), ..RunnerConfig::default() };
    assert_eq!(super::command(&bare, &m.probe, &m.exe, &m.game).expect("a command").args.last(), Some(&OsString::from(&m.exe)));
}

#[test]
fn a_custom_runner_with_no_command_or_a_broken_one_is_an_error_not_a_guess() {
    let m = machine();
    let none = RunnerConfig { kind: RunnerKind::Custom, ..RunnerConfig::default() };
    assert_eq!(command(&none, &m.probe, &m.exe, &m.game), Err(RunnerError::None));
    let broken = RunnerConfig { kind: RunnerKind::Custom, custom: Some("wine \"unclosed".into()), ..RunnerConfig::default() };
    assert!(matches!(command(&broken, &m.probe, &m.exe, &m.game), Err(RunnerError::BadCommand(_))));
    let empty = RunnerConfig { kind: RunnerKind::Custom, custom: Some("   ".into()), global_custom: None, ..RunnerConfig::default() };
    assert_eq!(command(&empty, &m.probe, &m.exe, &m.game), Err(RunnerError::None));
}

#[test]
fn asking_for_a_runner_that_is_not_there_names_it() {
    let m = machine();
    assert_eq!(
        command(&RunnerConfig { kind: RunnerKind::Wine, ..RunnerConfig::default() }, &m.probe, &m.exe, &m.game),
        Err(RunnerError::Missing("Wine"))
    );
    assert_eq!(
        command(&RunnerConfig { kind: RunnerKind::Proton, ..RunnerConfig::default() }, &m.probe, &m.exe, &m.game),
        Err(RunnerError::Missing("Proton"))
    );
}

#[test]
fn a_chosen_proton_and_prefix_are_used() {
    let m = machine();
    steam_proton(&m, "Proton 9.0");
    let mine = m.probe.home.join(".local/share/Steam/compatibilitytools.d/My-Proton/proton");
    touch(&mine, true);
    let prefix = m.game.join("my-prefix");
    let config =
        RunnerConfig { kind: RunnerKind::Proton, proton: Some(mine.clone()), prefix: Some(prefix.clone()), ..RunnerConfig::default() };
    let command = command(&config, &m.probe, &m.exe, &m.game).expect("a command");
    assert_eq!(command.program, mine);
    assert!(command.env.iter().any(|(k, v)| k == "STEAM_COMPAT_DATA_PATH" && v == prefix.as_os_str()));
    assert!(prefix.is_dir());
}

#[test]
fn commands_split_like_a_shell_would() {
    assert_eq!(split_command("a b  c").expect("split"), ["a", "b", "c"]);
    assert_eq!(split_command(r#"a "b c" 'd e' f\ g"#).expect("split"), ["a", "b c", "d e", "f g"]);
    assert_eq!(split_command(r#"a "" b"#).expect("split"), ["a", "", "b"], "an empty quoted word is a word");
    assert_eq!(split_command("").expect("split"), Vec::<String>::new());
    assert!(split_command("a 'b").is_err() && split_command("a \"b").is_err() && split_command("a\\").is_err());
}

#[test]
fn availability_follows_the_same_rules() {
    let m = machine();
    assert!(!available(&RunnerConfig::default(), &m.probe));
    steam_proton(&m, "Proton 9.0");
    assert!(available(&RunnerConfig::default(), &m.probe));
    assert!(!available(&RunnerConfig { kind: RunnerKind::Wine, ..RunnerConfig::default() }, &m.probe));
    assert!(!available(&RunnerConfig { kind: RunnerKind::Custom, ..RunnerConfig::default() }, &m.probe));
    assert!(available(&RunnerConfig { kind: RunnerKind::Custom, custom: Some("x".into()), ..RunnerConfig::default() }, &m.probe));
}
