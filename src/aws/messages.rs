use std::collections::HashSet;

use aws_sdk_rekognition::model::ModerationLabel;
use aws_sdk_rekognition::output::{DetectModerationLabelsOutput, GetContentModerationOutput};
use serde::Deserialize;

use crate::moderation::ModerationCategories;
use log::error;

#[derive(Deserialize, Default, Clone)]
#[allow(non_snake_case)]
pub struct Label {
    pub Confidence: Option<f32>,
    pub Name: Option<String>,
    pub ParentName: Option<String>,
}

impl Label {
    pub fn top_category(&self) -> &str {
        if let Some(name) = &self.ParentName {
            name
        } else if let Some(name) = &self.Name {
            name
        } else {
            ""
        }
    }

    pub fn get_labels_image(d: DetectModerationLabelsOutput) -> Labels {
        d.moderation_labels.map(|m| {
            let l: Vec<Label> = m
                .into_iter()
                .map(|m| {
                    let l: Label = m.into();
                    l
                })
                .collect();
            l
        })
    }

    pub fn get_labels_video(g: GetContentModerationOutput) -> Labels {
        g.moderation_labels.map(|m| {
            let l: Vec<Label> = m
                .into_iter()
                .map(|m| {
                    let l: Label = m.moderation_label.into();
                    l
                })
                .collect();
            l
        })
    }
}

impl From<ModerationLabel> for Label {
    fn from(m: ModerationLabel) -> Self {
        Self {
            Confidence: m.confidence,
            Name: m.name,
            ParentName: m.parent_name,
        }
    }
}

pub type Labels = Option<Vec<Label>>;

impl From<Option<ModerationLabel>> for Label {
    fn from(m: Option<ModerationLabel>) -> Self {
        match m {
            Some(m) => m.into(),
            _ => Default::default(),
        }
    }
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct RekognitionResponse {
    pub ModerationLabels: Vec<Label>,
    pub ModerationModelVersion: String,
}

impl From<DetectModerationLabelsOutput> for RekognitionResponse {
    fn from(d: DetectModerationLabelsOutput) -> Self {
        let model_ver = d
            .moderation_model_version
            .clone()
            .unwrap_or_else(|| "".into());
        let mod_labels: Option<Vec<Label>> = Label::get_labels_image(d);

        Self {
            ModerationLabels: mod_labels.unwrap_or_default(),
            ModerationModelVersion: model_ver,
        }
    }
}

impl From<GetContentModerationOutput> for RekognitionResponse {
    fn from(g: GetContentModerationOutput) -> Self {
        let model_ver = g
            .moderation_model_version
            .clone()
            .unwrap_or_else(|| "".into());
        let mod_labels: Option<Vec<Label>> = Label::get_labels_video(g);

        Self {
            ModerationLabels: mod_labels.unwrap_or_default(),
            ModerationModelVersion: model_ver,
        }
    }
}

impl RekognitionResponse {
    pub fn get_labels(&self) -> Vec<ModerationCategories> {
        let labels: HashSet<String> = self
            .ModerationLabels
            .iter()
            .map(|l| l.top_category().to_owned())
            .filter(|l| !l.is_empty())
            .collect();
        labels
            .iter()
            .map(|l| RekognitionResponse::normalize_category(l))
            .collect()
    }

    fn normalize_category(input: &str) -> ModerationCategories {
        match input {
            "Explicit Nudity" => ModerationCategories::ExplicitNudity,
            "Suggestive" => ModerationCategories::Suggestive,
            "Violence" => ModerationCategories::Violence,
            "Visually Disturbing" => ModerationCategories::VisuallyDisturbing,
            "Rude" => ModerationCategories::Rude,
            "Drugs" => ModerationCategories::Drugs,
            "Tobacco" => ModerationCategories::Tobacco,
            "Alcohol" => ModerationCategories::Alcohol,
            "Gambling" => ModerationCategories::Gambling,
            "Hate" => ModerationCategories::Hate,
            s => {
                error!("got and unknown category {}", s);

                ModerationCategories::Unknown
            }
        }
    }
}
