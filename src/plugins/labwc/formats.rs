use crate::config::ConfigEdit;

use super::environment;
use super::rc_xml;
use crate::config::{Applied, ConfigFormat};

pub struct RcXmlFormat;

impl ConfigFormat for RcXmlFormat {
    fn apply(&self, raw: &str, edits: &[ConfigEdit]) -> Result<Applied, String> {
        let mut contents = raw.to_string();
        let mut changed = Vec::with_capacity(edits.len());

        for edit in edits {
            let current = rc_xml::get(&contents, &edit.key);
            let unchanged = match &edit.value {
                Some(value) => current.as_deref() == Some(value.as_str()),
                None => current.is_none(),
            };
            if unchanged {
                changed.push(false);
                continue;
            }
            contents = match &edit.value {
                Some(value) => rc_xml::set(&contents, &edit.key, value)?,
                None => rc_xml::remove(&contents, &edit.key)?,
            };
            changed.push(true);
        }

        Ok(Applied { contents, changed })
    }
}

pub struct EnvironmentFormat;

impl ConfigFormat for EnvironmentFormat {
    fn apply(&self, raw: &str, edits: &[ConfigEdit]) -> Result<Applied, String> {
        let mut active = environment::parse_active(raw);
        let mut sets = std::collections::BTreeMap::new();
        let mut unsets = Vec::new();
        let mut changed = Vec::with_capacity(edits.len());

        for edit in edits {
            let current = active.get(&edit.key).cloned();
            let unchanged = match &edit.value {
                Some(value) => current.as_deref() == Some(value.as_str()),
                None => current.is_none(),
            };
            if unchanged {
                changed.push(false);
                continue;
            }
            match &edit.value {
                Some(value) => {
                    sets.insert(edit.key.clone(), value.clone());
                    active.insert(edit.key.clone(), value.clone());
                }
                None => {
                    if !unsets.iter().any(|key| key == &edit.key) {
                        unsets.push(edit.key.clone());
                    }
                    active.remove(&edit.key);
                }
            }
            changed.push(true);
        }

        let contents = if sets.is_empty() && unsets.is_empty() {
            raw.to_string()
        } else {
            environment::patch(raw, &sets, &unsets)
        };
        Ok(Applied { contents, changed })
    }
}

pub static RC_XML: RcXmlFormat = RcXmlFormat;
pub static ENVIRONMENT: EnvironmentFormat = EnvironmentFormat;
