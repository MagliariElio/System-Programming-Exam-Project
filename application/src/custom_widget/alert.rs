use druid::{Data, Lens};

#[derive(Clone, Data, Lens)]
pub struct Alert {
    pub(crate) alert_visible: bool,
    pub(crate) alert_message: String
}

impl Alert {
    pub fn show_alert(&mut self, message: &str) {
        self.alert_visible = true;
        self.alert_message = message.to_string();
    }

    pub fn hide_alert(&mut self) {
        self.alert_visible = false;
    }
}