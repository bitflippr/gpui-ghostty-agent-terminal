use crate::terminal_grid::font_pixels_to_points;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) use crate::terminal_theme::ThemePreset;

pub(crate) const MIN_FONT_SIZE: f32 = 8.;
pub(crate) const MAX_FONT_SIZE: f32 = 48.;
pub(crate) const MIN_OPACITY: f32 = 0.45;
pub(crate) const MAX_OPACITY: f32 = 1.;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum KeybindAction {
    OpenSettings,
    CreateSpace,
    CreateTab,
    ClosePane,
    SplitHorizontal,
    SplitVertical,
    NextTab,
    PreviousTab,
    NextSpace,
    PreviousSpace,
    CloseTab,
    CloseSpace,
    ToggleSidebar,
    FocusLeft,
    FocusRight,
    FocusUp,
    FocusDown,
    NextPane,
    PreviousPane,
    ResizeLeft,
    ResizeRight,
    ResizeUp,
    ResizeDown,
    IncreaseFont,
    DecreaseFont,
    ResetFont,
    ScrollLineUp,
    ScrollLineDown,
    ScrollPageUp,
    ScrollPageDown,
    ScrollTop,
    ScrollBottom,
    MovePane,
    NextAgent,
    PreviousAgent,
    ToggleAgentList,
    Quit,
    SelectTab1,
    SelectTab2,
    SelectTab3,
    SelectTab4,
    SelectTab5,
    SelectTab6,
    SelectTab7,
    SelectTab8,
    SelectTab9,
    SelectSpace1,
    SelectSpace2,
    SelectSpace3,
    SelectSpace4,
    SelectSpace5,
    SelectSpace6,
    SelectSpace7,
    SelectSpace8,
    SelectSpace9,
}

impl KeybindAction {
    pub(crate) const ALL: [Self; 55] = [
        Self::OpenSettings,
        Self::CreateSpace,
        Self::CreateTab,
        Self::ClosePane,
        Self::SplitHorizontal,
        Self::SplitVertical,
        Self::NextTab,
        Self::PreviousTab,
        Self::NextSpace,
        Self::PreviousSpace,
        Self::CloseTab,
        Self::CloseSpace,
        Self::ToggleSidebar,
        Self::FocusLeft,
        Self::FocusRight,
        Self::FocusUp,
        Self::FocusDown,
        Self::NextPane,
        Self::PreviousPane,
        Self::ResizeLeft,
        Self::ResizeRight,
        Self::ResizeUp,
        Self::ResizeDown,
        Self::IncreaseFont,
        Self::DecreaseFont,
        Self::ResetFont,
        Self::ScrollLineUp,
        Self::ScrollLineDown,
        Self::ScrollPageUp,
        Self::ScrollPageDown,
        Self::ScrollTop,
        Self::ScrollBottom,
        Self::MovePane,
        Self::NextAgent,
        Self::PreviousAgent,
        Self::ToggleAgentList,
        Self::Quit,
        Self::SelectTab1,
        Self::SelectTab2,
        Self::SelectTab3,
        Self::SelectTab4,
        Self::SelectTab5,
        Self::SelectTab6,
        Self::SelectTab7,
        Self::SelectTab8,
        Self::SelectTab9,
        Self::SelectSpace1,
        Self::SelectSpace2,
        Self::SelectSpace3,
        Self::SelectSpace4,
        Self::SelectSpace5,
        Self::SelectSpace6,
        Self::SelectSpace7,
        Self::SelectSpace8,
        Self::SelectSpace9,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::OpenSettings => "Open settings",
            Self::CreateSpace => "New Space",
            Self::CreateTab => "New tab",
            Self::ClosePane => "Close pane",
            Self::SplitHorizontal => "Split right",
            Self::SplitVertical => "Split down",
            Self::NextTab => "Next Tab",
            Self::PreviousTab => "Previous Tab",
            Self::NextSpace => "Next Space",
            Self::PreviousSpace => "Previous Space",
            Self::CloseTab => "Close Tab",
            Self::CloseSpace => "Close Space",
            Self::ToggleSidebar => "Toggle sidebar",
            Self::FocusLeft => "Focus Pane left",
            Self::FocusRight => "Focus Pane right",
            Self::FocusUp => "Focus Pane above",
            Self::FocusDown => "Focus Pane below",
            Self::NextPane => "Next Pane",
            Self::PreviousPane => "Previous Pane",
            Self::ResizeLeft => "Resize split left",
            Self::ResizeRight => "Resize split right",
            Self::ResizeUp => "Resize split up",
            Self::ResizeDown => "Resize split down",
            Self::IncreaseFont => "Increase font size",
            Self::DecreaseFont => "Decrease font size",
            Self::ResetFont => "Reset font size",
            Self::ScrollLineUp => "Scroll line up",
            Self::ScrollLineDown => "Scroll line down",
            Self::ScrollPageUp => "Scroll page up",
            Self::ScrollPageDown => "Scroll page down",
            Self::ScrollTop => "Scroll to top",
            Self::ScrollBottom => "Scroll to bottom",
            Self::MovePane => "Move Pane",
            Self::NextAgent => "Next agent",
            Self::PreviousAgent => "Previous agent",
            Self::ToggleAgentList => "Toggle agent list",
            Self::Quit => "Quit Application",
            Self::SelectTab1 => "Select Tab 1",
            Self::SelectTab2 => "Select Tab 2",
            Self::SelectTab3 => "Select Tab 3",
            Self::SelectTab4 => "Select Tab 4",
            Self::SelectTab5 => "Select Tab 5",
            Self::SelectTab6 => "Select Tab 6",
            Self::SelectTab7 => "Select Tab 7",
            Self::SelectTab8 => "Select Tab 8",
            Self::SelectTab9 => "Select Tab 9",
            Self::SelectSpace1 => "Select Space 1",
            Self::SelectSpace2 => "Select Space 2",
            Self::SelectSpace3 => "Select Space 3",
            Self::SelectSpace4 => "Select Space 4",
            Self::SelectSpace5 => "Select Space 5",
            Self::SelectSpace6 => "Select Space 6",
            Self::SelectSpace7 => "Select Space 7",
            Self::SelectSpace8 => "Select Space 8",
            Self::SelectSpace9 => "Select Space 9",
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::OpenSettings => "Show or hide this settings page",
            Self::CreateSpace => "Create a Space in the current directory",
            Self::CreateTab => "Create a tab in the selected Space",
            Self::ClosePane => "Close the focused pane",
            Self::SplitHorizontal => "Place a new pane to the right",
            Self::SplitVertical => "Place a new pane below",
            Self::NextTab => "Select the next Tab",
            Self::PreviousTab => "Select the previous Tab",
            Self::NextSpace => "Select the next Space",
            Self::PreviousSpace => "Select the previous Space",
            Self::CloseTab => "Close every Pane in the selected Tab",
            Self::CloseSpace => "Close every Tab in the selected Space",
            Self::ToggleSidebar => "Show or hide the Space sidebar",
            Self::FocusLeft => "Focus the nearest Pane to the left",
            Self::FocusRight => "Focus the nearest Pane to the right",
            Self::FocusUp => "Focus the nearest Pane above",
            Self::FocusDown => "Focus the nearest Pane below",
            Self::NextPane => "Cycle forward through Panes",
            Self::PreviousPane => "Cycle backward through Panes",
            Self::ResizeLeft => "Move the nearest vertical divider left",
            Self::ResizeRight => "Move the nearest vertical divider right",
            Self::ResizeUp => "Move the nearest horizontal divider up",
            Self::ResizeDown => "Move the nearest horizontal divider down",
            Self::IncreaseFont => "Increase terminal font size",
            Self::DecreaseFont => "Decrease terminal font size",
            Self::ResetFont => "Restore the default terminal font size",
            Self::ScrollLineUp => "Scroll terminal history up one line",
            Self::ScrollLineDown => "Scroll terminal history down one line",
            Self::ScrollPageUp => "Scroll terminal history up one page",
            Self::ScrollPageDown => "Scroll terminal history down one page",
            Self::ScrollTop => "Show the oldest terminal history",
            Self::ScrollBottom => "Return to live terminal output",
            Self::MovePane => "Choose a destination Pane using the keyboard",
            Self::NextAgent => "Focus the next agent terminal across Spaces",
            Self::PreviousAgent => "Focus the previous agent terminal across Spaces",
            Self::ToggleAgentList => "Expand or collapse agents in the selected Space",
            Self::Quit => "Stop all Terminal Sessions and quit",
            Self::SelectTab1 => "Select Tab 1 (9 selects the last)",
            Self::SelectTab2 => "Select Tab 2 (9 selects the last)",
            Self::SelectTab3 => "Select Tab 3 (9 selects the last)",
            Self::SelectTab4 => "Select Tab 4 (9 selects the last)",
            Self::SelectTab5 => "Select Tab 5 (9 selects the last)",
            Self::SelectTab6 => "Select Tab 6 (9 selects the last)",
            Self::SelectTab7 => "Select Tab 7 (9 selects the last)",
            Self::SelectTab8 => "Select Tab 8 (9 selects the last)",
            Self::SelectTab9 => "Select Tab 9 (9 selects the last)",
            Self::SelectSpace1 => "Select Space 1 (9 selects the last)",
            Self::SelectSpace2 => "Select Space 2 (9 selects the last)",
            Self::SelectSpace3 => "Select Space 3 (9 selects the last)",
            Self::SelectSpace4 => "Select Space 4 (9 selects the last)",
            Self::SelectSpace5 => "Select Space 5 (9 selects the last)",
            Self::SelectSpace6 => "Select Space 6 (9 selects the last)",
            Self::SelectSpace7 => "Select Space 7 (9 selects the last)",
            Self::SelectSpace8 => "Select Space 8 (9 selects the last)",
            Self::SelectSpace9 => "Select Space 9 (9 selects the last)",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Shortcut {
    pub(crate) key: String,
    #[serde(default)]
    pub(crate) control: bool,
    #[serde(default)]
    pub(crate) alt: bool,
    #[serde(default)]
    pub(crate) shift: bool,
    #[serde(default)]
    pub(crate) platform: bool,
}

impl Shortcut {
    pub(crate) fn is_usable(&self) -> bool {
        !self.key.trim().is_empty()
            && !matches!(
                self.key.to_ascii_lowercase().as_str(),
                "control" | "shift" | "alt" | "command" | "super" | "fn"
            )
    }

    pub(crate) fn display(&self) -> String {
        let mut parts = Vec::with_capacity(5);
        if self.control {
            parts.push("Ctrl".to_owned());
        }
        if self.alt {
            parts.push(
                if cfg!(target_os = "macos") {
                    "Option"
                } else {
                    "Alt"
                }
                .to_owned(),
            );
        }
        if self.shift {
            parts.push("Shift".to_owned());
        }
        if self.platform {
            parts.push(
                if cfg!(target_os = "macos") {
                    "Cmd"
                } else {
                    "Super"
                }
                .to_owned(),
            );
        }
        parts.push(display_key(&self.key));
        parts.join(" + ")
    }
}

fn display_key(key: &str) -> String {
    match key.to_ascii_lowercase().as_str() {
        "escape" => "Esc".to_owned(),
        "backspace" => "Backspace".to_owned(),
        "," => ",".to_owned(),
        key if key.len() == 1 => key.to_ascii_uppercase(),
        _ => key.to_owned(),
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct KeybindingSettings {
    #[serde(flatten)]
    bindings: std::collections::BTreeMap<KeybindAction, Option<Shortcut>>,
}

impl KeybindingSettings {
    pub(crate) fn get(&self, action: KeybindAction) -> Shortcut {
        self.custom(action)
            .cloned()
            .unwrap_or_else(|| default_shortcut(action))
    }

    pub(crate) fn custom(&self, action: KeybindAction) -> Option<&Shortcut> {
        self.bindings.get(&action).and_then(Option::as_ref)
    }

    pub(crate) fn set(&mut self, action: KeybindAction, shortcut: Option<Shortcut>) {
        if shortcut.is_some() {
            self.bindings.insert(action, shortcut);
        } else {
            self.bindings.remove(&action);
        }
    }

    pub(crate) fn conflict_for(
        &self,
        action: KeybindAction,
        shortcut: &Shortcut,
    ) -> Option<KeybindAction> {
        KeybindAction::ALL
            .into_iter()
            .find(|candidate| *candidate != action && self.get(*candidate) == *shortcut)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FontSizeUnit {
    Pixels,
    Points,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TerminalGlyphOverflow {
    #[default]
    WhenFollowedBySpace,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TerminalCursorStyle {
    #[default]
    #[serde(alias = "bar")]
    Beam,
    Block,
    Underline,
    HollowBlock,
}

impl TerminalCursorStyle {
    pub(crate) const ALL: [Self; 4] = [Self::Beam, Self::Block, Self::Underline, Self::HollowBlock];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Beam => "Beam",
            Self::Block => "Block",
            Self::Underline => "Underline",
            Self::HollowBlock => "Hollow block",
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::Beam => "A narrow vertical caret",
            Self::Block => "A filled cell that inverts its text",
            Self::Underline => "A line along the bottom of the cell",
            Self::HollowBlock => "An outline around the current cell",
        }
    }
}

impl From<TerminalCursorStyle> for crate::ghostty::CursorShape {
    fn from(style: TerminalCursorStyle) -> Self {
        match style {
            TerminalCursorStyle::Beam => Self::Bar,
            TerminalCursorStyle::Block => Self::Block,
            TerminalCursorStyle::Underline => Self::Underline,
            TerminalCursorStyle::HollowBlock => Self::BlockHollow,
        }
    }
}

impl TerminalGlyphOverflow {
    pub(crate) const ALL: [Self; 3] = [Self::WhenFollowedBySpace, Self::Always, Self::Never];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::WhenFollowedBySpace => "When safe",
            Self::Always => "Always",
            Self::Never => "Never",
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::WhenFollowedBySpace => "Keep symbols full-size when the next cell is blank",
            Self::Always => "Keep symbols full-size even beside other text",
            Self::Never => "Fit symbols to their terminal cell allocation",
        }
    }

    pub(crate) fn allows(self, followed_by_space: bool) -> bool {
        match self {
            Self::WhenFollowedBySpace => followed_by_space,
            Self::Always => true,
            Self::Never => false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct AppSettings {
    pub(crate) theme: ThemePreset,
    pub(crate) background_opacity: f32,
    pub(crate) font_family: Option<String>,
    pub(crate) font_size: f32,
    pub(crate) terminal_glyph_overflow: TerminalGlyphOverflow,
    pub(crate) cursor_style: TerminalCursorStyle,
    font_size_unit: FontSizeUnit,
    pub(crate) keybindings: KeybindingSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreset::TokyoNight,
            background_opacity: 0.65,
            font_family: None,
            font_size: font_pixels_to_points(14.),
            terminal_glyph_overflow: TerminalGlyphOverflow::default(),
            cursor_style: TerminalCursorStyle::Beam,
            font_size_unit: FontSizeUnit::Points,
            keybindings: KeybindingSettings::default(),
        }
    }
}

impl AppSettings {
    pub(crate) fn load() -> (Self, Option<String>) {
        let Some(path) = settings_path() else {
            return (Self::default(), None);
        };
        match fs::read_to_string(&path) {
            Ok(contents) => match Self::decode(&contents) {
                Ok(settings) => (settings, None),
                Err(error) => (
                    Self::default(),
                    Some(format!(
                        "ignored invalid settings at {}: {error}",
                        path.display()
                    )),
                ),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (Self::default(), None),
            Err(error) => (
                Self::default(),
                Some(format!(
                    "could not read settings at {}: {error}",
                    path.display()
                )),
            ),
        }
    }

    pub(crate) fn save(&self) -> Result<(), String> {
        let path = settings_path()
            .ok_or_else(|| "no user configuration directory is available".to_owned())?;
        let parent = path
            .parent()
            .ok_or_else(|| "settings path has no parent directory".to_owned())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("create settings directory {}: {error}", parent.display()))?;
        let contents = serde_json::to_vec_pretty(self)
            .map_err(|error| format!("serialize settings: {error}"))?;
        let temporary = path.with_extension("json.tmp");
        let mut file = fs::File::create(&temporary).map_err(|error| {
            format!("create temporary settings {}: {error}", temporary.display())
        })?;
        file.write_all(&contents)
            .and_then(|_| file.sync_all())
            .map_err(|error| {
                format!("write temporary settings {}: {error}", temporary.display())
            })?;
        drop(file);
        replace_file(&temporary, &path)
            .map_err(|error| format!("replace settings {}: {error}", path.display()))
    }

    fn decode(contents: &str) -> Result<Self, serde_json::Error> {
        let value = serde_json::from_str::<serde_json::Value>(contents)?;
        let legacy_pixels =
            value.get("font_size").is_some() && value.get("font_size_unit").is_none();
        let mut settings = serde_json::from_value::<Self>(value)?;
        if legacy_pixels || settings.font_size_unit == FontSizeUnit::Pixels {
            let legacy_size = if settings.font_size.is_finite() {
                settings.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
            } else {
                14.
            };
            settings.font_size = font_pixels_to_points(legacy_size);
            settings.font_size_unit = FontSizeUnit::Points;
        }
        settings.sanitize();
        Ok(settings)
    }

    pub(crate) fn effective_opacity(&self) -> f32 {
        std::env::var("AGENT_TERMINAL_BACKGROUND_OPACITY")
            .ok()
            .and_then(|value| parse_opacity(&value))
            .unwrap_or(self.background_opacity)
    }

    pub(crate) fn effective_font_size(&self) -> f32 {
        std::env::var("AGENT_TERMINAL_FONT_SIZE")
            .ok()
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|size| (MIN_FONT_SIZE..=MAX_FONT_SIZE).contains(size))
            .unwrap_or(self.font_size)
    }

    pub(crate) fn effective_font_family(&self) -> Option<String> {
        std::env::var("AGENT_TERMINAL_FONT")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| self.font_family.clone())
    }

    pub(crate) fn sanitize(&mut self) {
        self.background_opacity = if self.background_opacity.is_finite() {
            self.background_opacity.clamp(MIN_OPACITY, MAX_OPACITY)
        } else {
            Self::default().background_opacity
        };
        self.font_size = if self.font_size.is_finite() {
            self.font_size
                .clamp(minimum_persisted_font_size(), MAX_FONT_SIZE)
        } else {
            Self::default().font_size
        };
        if self
            .font_family
            .as_ref()
            .is_some_and(|family| family.trim().is_empty())
        {
            self.font_family = None;
        }
        for action in KeybindAction::ALL {
            if self
                .keybindings
                .custom(action)
                .is_some_and(|shortcut| !shortcut.is_usable())
            {
                self.keybindings.set(action, None);
            }
        }
    }
}

fn minimum_persisted_font_size() -> f32 {
    MIN_FONT_SIZE.min(font_pixels_to_points(MIN_FONT_SIZE))
}

pub(crate) fn adjust_font_size(font_size: f32, delta: f32) -> f32 {
    (font_size + delta).clamp(minimum_persisted_font_size(), MAX_FONT_SIZE)
}

pub(crate) fn default_shortcut(action: KeybindAction) -> Shortcut {
    default_shortcut_for(action, cfg!(target_os = "macos"))
}

fn default_shortcut_for(action: KeybindAction, macos: bool) -> Shortcut {
    let mut shortcut = Shortcut {
        key: match action {
            KeybindAction::OpenSettings => ",",
            KeybindAction::CreateSpace => "n",
            KeybindAction::CreateTab => "t",
            KeybindAction::ClosePane => "w",
            KeybindAction::SplitHorizontal => "d",
            KeybindAction::SplitVertical => "e",
            KeybindAction::NextTab => "tab",
            KeybindAction::PreviousTab => "tab",
            KeybindAction::NextSpace => "pagedown",
            KeybindAction::PreviousSpace => "pageup",
            KeybindAction::CloseTab => "w",
            KeybindAction::CloseSpace => "w",
            KeybindAction::ToggleSidebar => "b",
            KeybindAction::FocusLeft => "left",
            KeybindAction::FocusRight => "right",
            KeybindAction::FocusUp => "up",
            KeybindAction::FocusDown => "down",
            KeybindAction::NextPane => "]",
            KeybindAction::PreviousPane => "[",
            KeybindAction::ResizeLeft => "left",
            KeybindAction::ResizeRight => "right",
            KeybindAction::ResizeUp => "up",
            KeybindAction::ResizeDown => "down",
            KeybindAction::IncreaseFont => "=",
            KeybindAction::DecreaseFont => "-",
            KeybindAction::ResetFont => "0",
            KeybindAction::ScrollLineUp => "up",
            KeybindAction::ScrollLineDown => "down",
            KeybindAction::ScrollPageUp => "pageup",
            KeybindAction::ScrollPageDown => "pagedown",
            KeybindAction::ScrollTop => "home",
            KeybindAction::ScrollBottom => "end",
            KeybindAction::MovePane => "m",
            KeybindAction::NextAgent => "u",
            KeybindAction::PreviousAgent => "u",
            KeybindAction::ToggleAgentList => ".",
            KeybindAction::Quit => "q",
            KeybindAction::SelectTab1 => "1",
            KeybindAction::SelectTab2 => "2",
            KeybindAction::SelectTab3 => "3",
            KeybindAction::SelectTab4 => "4",
            KeybindAction::SelectTab5 => "5",
            KeybindAction::SelectTab6 => "6",
            KeybindAction::SelectTab7 => "7",
            KeybindAction::SelectTab8 => "8",
            KeybindAction::SelectTab9 => "9",
            KeybindAction::SelectSpace1 => "1",
            KeybindAction::SelectSpace2 => "2",
            KeybindAction::SelectSpace3 => "3",
            KeybindAction::SelectSpace4 => "4",
            KeybindAction::SelectSpace5 => "5",
            KeybindAction::SelectSpace6 => "6",
            KeybindAction::SelectSpace7 => "7",
            KeybindAction::SelectSpace8 => "8",
            KeybindAction::SelectSpace9 => "9",
        }
        .to_owned(),
        control: !macos,
        alt: false,
        shift: false,
        platform: macos,
    };
    shortcut.shift = matches!(
        action,
        KeybindAction::CreateSpace
            | KeybindAction::ClosePane
            | KeybindAction::SplitHorizontal
            | KeybindAction::SplitVertical
    );
    use KeybindAction::*;
    match action {
        NextTab | PreviousTab => {
            shortcut.control = true;
            shortcut.platform = false;
            shortcut.shift = action == PreviousTab;
        }
        NextSpace | PreviousSpace => {
            shortcut.alt = true;
        }
        CloseTab => {
            shortcut.alt = true;
            shortcut.shift = true;
        }
        CloseSpace => {
            shortcut.alt = true;
        }
        FocusLeft | FocusRight | FocusUp | FocusDown => {
            shortcut.alt = true;
        }
        ResizeLeft | ResizeRight | ResizeUp | ResizeDown => {
            shortcut.alt = true;
            shortcut.shift = true;
        }
        ScrollPageUp | ScrollPageDown | ScrollTop | ScrollBottom => {
            shortcut.control = false;
            shortcut.platform = false;
            shortcut.shift = true;
        }
        ToggleSidebar | NextPane | PreviousPane | ScrollLineUp | ScrollLineDown | MovePane
        | NextAgent | ToggleAgentList => {
            shortcut.shift = true;
        }
        PreviousAgent => {
            shortcut.shift = true;
            shortcut.alt = true;
        }
        Quit => {
            shortcut.shift = !macos;
        }
        SelectTab1 | SelectTab2 | SelectTab3 | SelectTab4 | SelectTab5 | SelectTab6
        | SelectTab7 | SelectTab8 | SelectTab9 => {
            shortcut.control = false;
            shortcut.platform = macos;
            shortcut.alt = !macos;
        }
        SelectSpace1 | SelectSpace2 | SelectSpace3 | SelectSpace4 | SelectSpace5 | SelectSpace6
        | SelectSpace7 | SelectSpace8 | SelectSpace9 => {
            shortcut.control = false;
            shortcut.platform = macos;
            shortcut.alt = !macos;
            shortcut.shift = true;
        }
        _ => {}
    }
    shortcut
}

pub(crate) fn parse_opacity(value: &str) -> Option<f32> {
    let opacity = value.trim().parse::<f32>().ok()?;
    opacity
        .is_finite()
        .then(|| opacity.clamp(MIN_OPACITY, MAX_OPACITY))
}

fn settings_path() -> Option<PathBuf> {
    if let Some(directory) =
        std::env::var_os("AGENT_TERMINAL_CONFIG_DIR").filter(|value| !value.is_empty())
    {
        return Some(PathBuf::from(directory).join("settings.json"));
    }
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .map(|directory| directory.join("agent-terminal").join("settings.json"))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .filter(|value| !value.is_empty())
                    .map(PathBuf::from)
                    .map(|home| home.join(".config"))
            })
            .map(|directory| directory.join("agent-terminal").join("settings.json"))
    }
}

#[cfg(windows)]
fn replace_file(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{REPLACEFILE_WRITE_THROUGH, ReplaceFileW};

    if !destination.exists() {
        return fs::rename(temporary, destination);
    }

    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let temporary = temporary
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let replaced = unsafe {
        ReplaceFileW(
            destination.as_ptr(),
            temporary.as_ptr(),
            std::ptr::null(),
            REPLACEFILE_WRITE_THROUGH,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(temporary, destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_defaults_are_unique_on_each_platform() {
        for macos in [false, true] {
            for (index, action) in KeybindAction::ALL.into_iter().enumerate() {
                let binding = default_shortcut_for(action, macos);
                assert!(binding.is_usable());
                for other in &KeybindAction::ALL[index + 1..] {
                    assert_ne!(
                        binding,
                        default_shortcut_for(*other, macos),
                        "{action:?} conflicts with {other:?} (macOS={macos})"
                    );
                }
            }
        }
    }

    #[test]
    fn keyboard_settings_preserve_legacy_bindings_and_round_trip_new_ones() {
        let mut settings: KeybindingSettings =
            serde_json::from_str(r#"{"create_tab":{"key":"k","control":true},"close_pane":null}"#)
                .unwrap();
        assert_eq!(settings.get(KeybindAction::CreateTab).key, "k");
        assert_eq!(
            settings.get(KeybindAction::ClosePane),
            default_shortcut(KeybindAction::ClosePane)
        );
        let custom = Shortcut {
            key: "f8".into(),
            control: false,
            alt: false,
            shift: false,
            platform: false,
        };
        settings.set(KeybindAction::NextSpace, Some(custom.clone()));
        let decoded: KeybindingSettings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(decoded.get(KeybindAction::NextSpace), custom);
        assert_eq!(decoded.get(KeybindAction::CreateTab).key, "k");
    }

    #[test]
    fn defaults_round_trip_and_missing_fields_are_filled() {
        let serialized = serde_json::to_string(&AppSettings::default()).unwrap();
        let decoded: AppSettings = serde_json::from_str(&serialized).unwrap();
        assert_eq!(decoded.theme, ThemePreset::TokyoNight);
        let migrated: AppSettings = serde_json::from_str(r#"{"theme":"midnight"}"#).unwrap();
        assert_eq!(migrated.theme, ThemePreset::TokyoNight);
        let partial: AppSettings = serde_json::from_str(r#"{"theme":"nord"}"#).unwrap();
        assert_eq!(partial.theme, ThemePreset::Nord);
        assert_eq!(partial.font_size, font_pixels_to_points(14.));
        assert_eq!(
            partial.terminal_glyph_overflow,
            TerminalGlyphOverflow::WhenFollowedBySpace
        );
        assert_eq!(partial.cursor_style, TerminalCursorStyle::Beam);
    }

    #[test]
    fn glyph_overflow_policy_round_trips_all_supported_values() {
        for policy in TerminalGlyphOverflow::ALL {
            let settings = AppSettings {
                terminal_glyph_overflow: policy,
                ..AppSettings::default()
            };

            let serialized = serde_json::to_string(&settings).unwrap();
            let decoded: AppSettings = serde_json::from_str(&serialized).unwrap();

            assert_eq!(decoded.terminal_glyph_overflow, policy);
        }
    }

    #[test]
    fn cursor_style_defaults_to_beam_and_round_trips_all_supported_values() {
        assert_eq!(
            AppSettings::default().cursor_style,
            TerminalCursorStyle::Beam
        );
        for cursor_style in TerminalCursorStyle::ALL {
            let settings = AppSettings {
                cursor_style,
                ..AppSettings::default()
            };

            let serialized = serde_json::to_string(&settings).unwrap();
            let decoded: AppSettings = serde_json::from_str(&serialized).unwrap();

            assert_eq!(decoded.cursor_style, cursor_style);
        }
    }

    #[test]
    fn legacy_pixel_font_sizes_migrate_to_points_once() {
        let migrated = AppSettings::decode(r#"{"font_size":14}"#).unwrap();
        assert_eq!(migrated.font_size, font_pixels_to_points(14.));
        assert_eq!(migrated.font_size_unit, FontSizeUnit::Points);

        let serialized = serde_json::to_string(&migrated).unwrap();
        let decoded = AppSettings::decode(&serialized).unwrap();
        assert_eq!(decoded.font_size, font_pixels_to_points(14.));
    }

    #[test]
    fn legacy_minimum_pixel_size_survives_point_migration() {
        let migrated = AppSettings::decode(r#"{"font_size":8}"#).unwrap();
        assert_eq!(migrated.font_size, font_pixels_to_points(8.));

        assert_eq!(
            adjust_font_size(migrated.font_size, -1.),
            migrated.font_size
        );

        let serialized = serde_json::to_string(&migrated).unwrap();
        let decoded = AppSettings::decode(&serialized).unwrap();
        assert_eq!(decoded.font_size, font_pixels_to_points(8.));
    }

    #[test]
    fn legacy_pixel_bounds_are_applied_before_conversion() {
        let below = AppSettings::decode(r#"{"font_size":1}"#).unwrap();
        let above = AppSettings::decode(r#"{"font_size":100}"#).unwrap();

        assert_eq!(below.font_size, font_pixels_to_points(MIN_FONT_SIZE));
        assert_eq!(above.font_size, font_pixels_to_points(MAX_FONT_SIZE));
    }

    #[test]
    fn invalid_numeric_preferences_are_sanitized() {
        let mut settings = AppSettings {
            background_opacity: f32::NAN,
            font_size: 100.,
            ..AppSettings::default()
        };
        settings.sanitize();
        assert_eq!(settings.background_opacity, 0.65);
        assert_eq!(settings.font_size, MAX_FONT_SIZE);
        assert_eq!(parse_opacity("0"), Some(MIN_OPACITY));
        assert_eq!(parse_opacity("NaN"), None);
    }

    #[test]
    fn unusable_persisted_shortcuts_are_reset_to_defaults() {
        let mut settings: AppSettings = serde_json::from_str(
            r#"{"keybindings":{"open_settings":{"key":""},"create_tab":{"key":"control"}}}"#,
        )
        .unwrap();

        settings.sanitize();

        assert_eq!(
            settings.keybindings.get(KeybindAction::OpenSettings),
            default_shortcut(KeybindAction::OpenSettings)
        );
        assert_eq!(
            settings.keybindings.get(KeybindAction::CreateTab),
            default_shortcut(KeybindAction::CreateTab)
        );
        assert!(
            settings
                .keybindings
                .custom(KeybindAction::OpenSettings)
                .is_none()
        );
        assert!(
            settings
                .keybindings
                .custom(KeybindAction::CreateTab)
                .is_none()
        );
    }

    #[test]
    fn custom_shortcuts_override_and_reset_to_defaults() {
        let mut keybindings = KeybindingSettings::default();
        let custom = Shortcut {
            key: "k".to_owned(),
            control: true,
            alt: true,
            shift: false,
            platform: false,
        };
        keybindings.set(KeybindAction::CreateTab, Some(custom.clone()));
        assert_eq!(keybindings.get(KeybindAction::CreateTab), custom);
        keybindings.set(KeybindAction::CreateTab, None);
        assert_eq!(
            keybindings.get(KeybindAction::CreateTab),
            default_shortcut(KeybindAction::CreateTab)
        );
    }

    #[test]
    fn resetting_reports_a_conflict_with_the_default_shortcut() {
        let mut keybindings = KeybindingSettings::default();
        let create_tab_default = default_shortcut(KeybindAction::CreateTab);
        keybindings.set(
            KeybindAction::CreateTab,
            Some(Shortcut {
                key: "k".to_owned(),
                ..create_tab_default.clone()
            }),
        );
        keybindings.set(
            KeybindAction::OpenSettings,
            Some(create_tab_default.clone()),
        );

        assert_eq!(
            keybindings.conflict_for(KeybindAction::CreateTab, &create_tab_default),
            Some(KeybindAction::OpenSettings)
        );
    }

    #[cfg(windows)]
    #[test]
    fn replacing_an_existing_settings_file_installs_the_complete_new_file() {
        let directory = std::env::temp_dir().join(format!(
            "agent-terminal-settings-replace-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let destination = directory.join("settings.json");
        let temporary = directory.join("settings.json.tmp");
        fs::write(&destination, b"old").unwrap();
        fs::write(&temporary, b"new").unwrap();

        replace_file(&temporary, &destination).unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"new");
        assert!(!temporary.exists());
        fs::remove_dir_all(directory).unwrap();
    }
}
