use std::collections::HashSet;

use serde::Deserialize;

use crate::moderation::ModerationCategories;

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct Label {
    pub Confidence: f32,
    pub Name: String,
    pub ParentName: String,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct RekognitionResponse {
    pub ModerationLabels: Vec<Label>,
    pub ModerationModelVersion: String,
}

impl RekognitionResponse {
    pub fn get_labels(&self) -> Vec<ModerationCategories> {
        let labels: HashSet<String> = self
            .ModerationLabels
            .iter()
            .map(|l| l.ParentName.clone())
            .filter(|l| l.len() > 0)
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