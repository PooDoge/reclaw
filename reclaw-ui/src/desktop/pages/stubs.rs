use freya::prelude::*;

macro_rules! stub { ($name:ident { $($f:ident: $t:ty),* }) => {
    #[derive(PartialEq)]
    pub struct $name { $(pub $f: $t),* }
    impl Component for $name { fn render(&self) -> impl IntoElement { label().text(stringify!($name)) } }
}; }
stub!(GameSettingsPage { id: u32, section: Option<String> });
stub!(SettingsPage { section: Option<String> });
