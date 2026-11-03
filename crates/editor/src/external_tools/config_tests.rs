//! The config's contract: how a request resolves — override, category, project, the
//! system default — what each error names, the preset's program, the legacy IDE command,
//! and the JSON round trip.

use super::*;

const FILE: &str = "/game/assets/scripts/paddle.rhai";
const PROJECT: &str = "/game";

fn resolve_at(config: &ExternalToolsConfig, file_type: FileType, location: &Location, platform: Platform) -> Result<Resolution, ResolveError> {
    let target = LaunchTarget { file_type, file: Path::new(FILE), project: Path::new(PROJECT), location };
    resolve(config, target, platform)
}

fn tool(program: &str, args: &[&str], preset: Option<PresetId>) -> Resolution {
    Resolution::Tool(Launch {
        program: program.to_string(),
        args: args.iter().map(|argument| argument.to_string()).collect(),
        preset,
        named_program: false,
    })
}

fn custom(file: &str) -> ToolChoice {
    ToolChoice::Custom(CustomTool::file_only(CommandTemplate(file.to_string())))
}

fn xdg_open() -> Resolution {
    Resolution::OsDefault(OsDefault::Program(Launch {
        program: "xdg-open".to_string(),
        args: vec![FILE.to_string()],
        preset: None,
        named_program: false,
    }))
}

#[test]
fn test_an_absent_override_inherits_the_category_and_a_present_one_wins_even_as_os_default() {
    let mut config = ExternalToolsConfig { music: ToolChoice::Preset(PresetId::MidiEvil), ..Default::default() };
    let midi = resolve_at(&config, FileType::Midi, &Location::None, Platform::Linux);
    assert_eq!(midi, Ok(tool("midi-evil", &["--project", PROJECT, FILE], Some(PresetId::MidiEvil))));

    config.overrides.insert(FileType::SoundFont, ToolChoice::OsDefault);
    assert_eq!(
        resolve_at(&config, FileType::SoundFont, &Location::None, Platform::Linux),
        Ok(xdg_open()),
        "an explicit OsDefault override is honoured over the Music category's tool"
    );
    assert_eq!(
        resolve_at(&config, FileType::DawProject, &Location::None, Platform::Linux).map(|resolution| matches!(resolution, Resolution::Tool(_))),
        Ok(true),
        "a type without an override still inherits the category"
    );
}

#[test]
fn test_code_left_at_the_system_default_opens_through_the_platform_opener_at_no_line() {
    let at_line = Location::Line { line: 12, column: None };
    assert_eq!(
        resolve_at(&ExternalToolsConfig::default(), FileType::RhaiScript, &at_line, Platform::Linux),
        Ok(xdg_open()),
        "no terminal editor from the environment stands in for the system default"
    );
}

#[test]
fn test_each_platforms_default_is_its_own_opener() {
    let config = ExternalToolsConfig::default();
    let on = |platform| resolve_at(&config, FileType::Image, &Location::None, platform);
    assert_eq!(on(Platform::Linux), Ok(xdg_open()));
    assert_eq!(
        on(Platform::MacOs),
        Ok(Resolution::OsDefault(OsDefault::Program(Launch {
            program: "open".to_string(),
            args: vec![FILE.to_string()],
            preset: None,
            named_program: false
        })))
    );
    assert_eq!(on(Platform::Windows), Ok(Resolution::OsDefault(OsDefault::ShellAssociation(PathBuf::from(FILE)))));
    assert_eq!(on(Platform::Wsl), Ok(Resolution::OsDefault(OsDefault::WslView(PathBuf::from(FILE)))));
}

#[test]
fn test_a_project_opens_through_the_presets_project_template_and_nothing_else() {
    let mut config = ExternalToolsConfig { code: ToolChoice::Preset(PresetId::RustRover), ..Default::default() };
    assert_eq!(
        resolve_at(&config, FileType::RustSource, &Location::Project, Platform::Linux),
        Ok(tool("rustrover", &[PROJECT], Some(PresetId::RustRover)))
    );
    config.code = custom("edit {file}");
    assert!(matches!(
        resolve_at(&config, FileType::RustSource, &Location::Project, Platform::Linux),
        Err(ResolveError::NoProjectCommand { .. })
    ));
    let project = Some(CommandTemplate("edit --folder {project}".to_string()));
    config.code = ToolChoice::Custom(CustomTool { file: CommandTemplate("edit {file}".to_string()), project });
    assert_eq!(
        resolve_at(&config, FileType::RustSource, &Location::Project, Platform::Linux),
        Ok(tool("edit", &["--folder", PROJECT], None)),
        "a custom tool's project command opens the project"
    );
    config.code = ToolChoice::OsDefault;
    assert!(matches!(
        resolve_at(&config, FileType::RustSource, &Location::Project, Platform::Linux),
        Err(ResolveError::NoProjectCommand { .. })
    ));
}

#[test]
fn test_a_custom_command_with_an_unknown_placeholder_is_refused_naming_it() {
    let config = ExternalToolsConfig { code: custom("edit {path}"), ..Default::default() };
    assert_eq!(
        resolve_at(&config, FileType::RhaiScript, &Location::None, Platform::Linux),
        Err(ResolveError::Template {
            tool: "The custom command".to_string(),
            error: TemplateError::UnknownPlaceholder("path".to_string())
        })
    );
}

#[test]
fn test_a_track_location_fills_the_track_placeholders() {
    let config = ExternalToolsConfig { music: ToolChoice::Preset(PresetId::MidiEvil), ..Default::default() };
    let track = Location::Track { name: Some("drums".to_string()), index: 2 };
    assert_eq!(
        resolve_at(&config, FileType::Midi, &track, Platform::Linux),
        Ok(tool("midi-evil", &["--project", PROJECT, "--track", "drums", "--track-index", "2", FILE], Some(PresetId::MidiEvil)))
    );
}

#[test]
fn test_the_open_item_names_what_the_file_resolves_to_as_resolves_messages_do() {
    let mut config = ExternalToolsConfig {
        code: ToolChoice::Preset(PresetId::RustRover),
        music: custom("player {file}"),
        ..Default::default()
    };
    config.overrides.insert(FileType::Image, ToolChoice::Preset(PresetId::Aseprite));
    let items: Vec<String> = [FileType::RhaiScript, FileType::Image, FileType::Midi, FileType::AudioSample]
        .into_iter()
        .map(|file_type| config.tool_name(file_type).open_item())
        .collect();
    assert_eq!(
        items,
        ["Open in RustRover", "Open in Aseprite", "Open with the custom command", "Open with the system default"],
        "a preset, an override, Custom and OsDefault"
    );
    config.music = ToolChoice::Preset(PresetId::MidiEvil);
    assert_eq!(config.tool_name(FileType::Midi).edit_item(), "Edit in MIDI Evil", "the emitter's button, by the same name");

    let refusal = |choice: ToolChoice| {
        let config = ExternalToolsConfig { code: choice.clone(), ..Default::default() };
        let Err(error) = resolve_at(&config, FileType::RustSource, &Location::Project, Platform::Linux) else {
            panic!("{choice:?} opens no project here");
        };
        (error.to_string(), ToolName::of(&choice).subject())
    };
    for choice in [custom("edit {file}"), ToolChoice::OsDefault] {
        let (message, subject) = refusal(choice);
        assert!(message.starts_with(subject), "the error names the tool as the menu does: {message}");
    }
}

#[test]
fn test_a_legacy_ide_command_becomes_the_custom_code_tool_with_the_file_appended() {
    let mut config = ExternalToolsConfig::default();
    config.adopt_legacy_ide_command("code --wait");
    assert_eq!(config.code, custom("code --wait {file}"));

    let mut placed = ExternalToolsConfig::default();
    placed.adopt_legacy_ide_command(" subl {file} --new-window ");
    assert_eq!(placed.code, custom("subl {file} --new-window"));

    let mut windows = ExternalToolsConfig::default();
    windows.adopt_legacy_ide_command(r"C:\tools\subl.exe -w");
    let ToolChoice::Custom(CustomTool { file: template, .. }) = &windows.code else {
        panic!("a legacy command becomes the custom code tool, not {:?}", windows.code);
    };
    let values = PlaceholderValues { file: "a.rhai".to_string(), ..Default::default() };
    assert_eq!(
        template.expand(&values),
        Ok(vec![r"C:\tools\subl.exe".to_string(), "-w".to_string(), "a.rhai".to_string()]),
        "the old split kept every backslash, so the carried command does too"
    );

    let embedded = |command: &str| {
        let mut config = ExternalToolsConfig::default();
        config.adopt_legacy_ide_command(command);
        match &config.code {
            ToolChoice::Custom(tool) => tool.file.expand(&values),
            other => panic!("a legacy command becomes the custom code tool, not {other:?}"),
        }
    };
    assert_eq!(
        embedded("code --goto {file}:12"),
        Ok(vec!["code".to_string(), "--goto".to_string(), "a.rhai:12".to_string()]),
        "a placeholder inside a word is still the placeholder, and no second file is appended"
    );
    assert_eq!(embedded("tool --file={file}"), Ok(vec!["tool".to_string(), "--file=a.rhai".to_string()]));
    assert_eq!(
        embedded(r#"tool "{file}""#),
        Ok(vec!["tool".to_string(), r#""a.rhai""#.to_string()]),
        "the old split passed the quote characters through, and so does the carried word"
    );

    let mut blank = ExternalToolsConfig::default();
    blank.adopt_legacy_ide_command("   \t ");
    assert_eq!(blank.code, ToolChoice::OsDefault, "a blank command chooses nothing");

    let mut chosen = ExternalToolsConfig { code: ToolChoice::Preset(PresetId::Zed), ..Default::default() };
    chosen.adopt_legacy_ide_command("code --wait");
    assert_eq!(chosen.code, ToolChoice::Preset(PresetId::Zed), "a chosen code tool is never replaced by the old string");
}

#[test]
fn test_a_project_resolves_through_the_code_categorys_choice_past_an_override_on_rust_source() {
    let mut config = ExternalToolsConfig { code: ToolChoice::Preset(PresetId::RustRover), ..Default::default() };
    config.overrides.insert(FileType::RustSource, custom("hx {file}"));
    assert_eq!(
        resolve_at(&config, FileType::RustSource, &Location::None, Platform::Linux),
        Ok(tool("hx", &[FILE], None)),
        "a Rust file opens through its override"
    );
    assert_eq!(
        resolve_at(&config, FileType::RustSource, &Location::Project, Platform::Linux),
        Ok(tool("rustrover", &[PROJECT], Some(PresetId::RustRover))),
        "the project opens through the category's tool, which an override on a file type never masks"
    );
}

#[test]
fn test_resolve_with_tries_the_given_choice_past_the_files_override() {
    let mut config = ExternalToolsConfig { art: ToolChoice::Preset(PresetId::Aseprite), ..Default::default() };
    config.overrides.insert(FileType::Image, ToolChoice::OsDefault);
    assert_eq!(resolve_at(&config, FileType::Image, &Location::None, Platform::Linux), Ok(xdg_open()), "a file honours its override");
    let target = LaunchTarget { file_type: FileType::Image, file: Path::new(FILE), project: Path::new(PROJECT), location: &Location::None };
    assert_eq!(
        resolve_with(&config, config.category_choice(ToolCategory::Art), target, Platform::Linux),
        Ok(tool("aseprite", &[FILE], Some(PresetId::Aseprite))),
        "a Test of the Art row tries Aseprite whatever the Image override says"
    );
}

#[test]
fn test_a_presets_program_replaces_the_templates_own_as_a_named_program_with_the_preset_kept() {
    let mut config = ExternalToolsConfig { music: ToolChoice::Preset(PresetId::MidiEvil), ..Default::default() };
    let own = resolve_at(&config, FileType::Midi, &Location::None, Platform::Linux);
    assert_eq!(own, Ok(tool("midi-evil", &["--project", PROJECT, FILE], Some(PresetId::MidiEvil))), "an absent entry keeps the template's own");

    config.programs.insert(PresetId::MidiEvil, "  ".to_string());
    assert_eq!(resolve_at(&config, FileType::Midi, &Location::None, Platform::Linux), own, "a blank entry is no entry");

    config.programs.insert(PresetId::MidiEvil, "/opt/midi-evil/target/release/midi-evil".to_string());
    let Ok(Resolution::Tool(own)) = own else {
        panic!("the preset's own resolves to a tool, not {own:?}");
    };
    assert_eq!(
        resolve_at(&config, FileType::Midi, &Location::None, Platform::Linux),
        Ok(Resolution::Tool(Launch { program: "/opt/midi-evil/target/release/midi-evil".to_string(), named_program: true, ..own.clone() })),
        "the named program runs with the preset kept for its diagnostics, marked named so the launcher skips the preset's fallback locations"
    );

    config.programs.insert(PresetId::MidiEvil, "midi-evil".to_string());
    assert_eq!(
        resolve_at(&config, FileType::Midi, &Location::None, Platform::Linux),
        Ok(Resolution::Tool(own.clone())),
        "the preset's own name typed in the field is its own program, fallback locations and all"
    );
    config.programs.insert(PresetId::MidiEvil, "Midi-Evil.exe".to_string());
    assert_eq!(
        resolve_at(&config, FileType::Midi, &Location::None, Platform::Linux),
        Ok(Resolution::Tool(Launch { program: "Midi-Evil.exe".to_string(), ..own.clone() })),
        "so is its own name with Windows' extension and casing"
    );
    config.programs.insert(PresetId::MidiEvil, "midi-evil.beta".to_string());
    assert_eq!(
        resolve_at(&config, FileType::Midi, &Location::None, Platform::Linux),
        Ok(Resolution::Tool(Launch { program: "midi-evil.beta".to_string(), named_program: true, ..own })),
        "any other suffix is another program: the fallbacks would run the preset's binary under its name"
    );
}

#[test]
fn test_an_empty_custom_command_is_refused_naming_no_program_and_never_panics() {
    let config = ExternalToolsConfig { code: custom(""), ..Default::default() };
    assert_eq!(
        resolve_at(&config, FileType::RhaiScript, &Location::None, Platform::Linux),
        Err(ResolveError::Template { tool: "The custom command".to_string(), error: TemplateError::NoProgram }),
        "the empty command a fresh Custom row starts with — a Test or a double-click before one is typed"
    );
}

#[test]
fn test_the_config_round_trips_through_json_and_a_partial_one_fills_defaults() -> Result<(), serde_json::Error> {
    let mut config = ExternalToolsConfig { art: ToolChoice::Preset(PresetId::Aseprite), ..Default::default() };
    config.overrides.insert(FileType::SoundFont, ToolChoice::OsDefault);
    config.overrides.insert(FileType::Ron, custom("hx {file}"));
    config.programs.insert(PresetId::MidiEvil, "/opt/midi-evil/midi-evil".to_string());
    let json = serde_json::to_string(&config)?;
    assert_eq!(serde_json::from_str::<ExternalToolsConfig>(&json)?, config);

    let partial: ExternalToolsConfig = serde_json::from_str(r#"{"code": {"Preset": "Zed"}}"#)?;
    assert_eq!(partial.code, ToolChoice::Preset(PresetId::Zed));
    assert_eq!(partial.daw_project_extensions, ExternalToolsConfig::default().daw_project_extensions);
    Ok(())
}
