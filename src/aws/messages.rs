use std::collections::HashSet;

use serde::Deserialize;
use aws_sdk_rekognition::model::ModerationLabel;
use aws_sdk_rekognition::output::DetectModerationLabelsOutput;

use crate::moderation::ModerationCategories;

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct Label {
    pub Confidence: f32,
    pub Name: String,
    pub ParentName: String,
}

impl From<ModerationLabel> for Label {
    fn from(m: ModerationLabel) -> Self {
        Self {
            Confidence: m.confidence.unwrap_or(-1.0),
            Name: m.name.unwrap_or("".into()),
            ParentName: m.parent_name.unwrap_or("".into()),
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
        Self {
            ModerationLabels: d
                .moderation_labels
                .unwrap()
                .into_iter()
                .map(|m| {
                    let l: Label = m.into();
                    l
                })
                .collect(),
            ModerationModelVersion: d.moderation_model_version.unwrap_or("".into()),
        }
    }
}




impl RekognitionResponse {
    pub fn get_labels(&self) -> Vec<ModerationCategories> {
        let labels: HashSet<String> = self
            .ModerationLabels
            .iter()
            .map(|l| l.ParentName.clone())
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
            _ => ModerationCategories::Unknown,
        }
    }
}
