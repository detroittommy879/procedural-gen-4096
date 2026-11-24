use std::{fs, io, path::Path};

use serde::Deserialize;

use crate::{
    gpu::{Effects, RenderInput, Shot},
    scenes::Scene,
};

const BUNDLED_STORY: &str = include_str!("../scenes/story.json");

#[derive(Deserialize)]
pub struct Timeline {
    pub title: String,
    pub premise: String,
    pub chapters: Vec<Chapter>,
}

#[derive(Deserialize)]
pub struct Chapter {
    pub title: String,
    pub scene: String,
    pub seconds: u32,
    #[serde(default)]
    action: Action,
    seed: Option<u32>,
    light: Option<[f32; 2]>,
    #[serde(default)]
    cues: Vec<Cue>,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[repr(u32)]
enum Action {
    #[default]
    None = 0,
    Blackout = 1,
    Listen = 2,
    Cross = 3,
    Float = 4,
    Recover = 5,
    Return = 6,
    Relight = 7,
    Synchronize = 8,
}

#[derive(Deserialize)]
struct Cue {
    at: u32,
    text: String,
    #[serde(default)]
    camera: Option<CameraMove>,
}

#[derive(Deserialize)]
struct CameraMove {
    from: [f32; 3],
    to: [f32; 3],
}

impl Chapter {
    fn camera_at(&self, elapsed: f32) -> [f32; 3] {
        let count = self.cues.partition_point(|cue| cue.at as f32 <= elapsed);
        let Some(cue) = count.checked_sub(1).map(|i| &self.cues[i]) else {
            return [0.0, 0.0, 1.0];
        };
        let Some(camera) = &cue.camera else {
            return [0.0, 0.0, 1.0];
        };
        let end = self.cues.get(count).map_or(self.seconds, |next| next.at);
        let t = ((elapsed - cue.at as f32) / (end - cue.at) as f32).clamp(0.0, 1.0);
        let t = t * t * (3.0 - 2.0 * t);
        std::array::from_fn(|i| camera.from[i] + (camera.to[i] - camera.from[i]) * t)
    }
}

pub struct Moment<'a> {
    pub chapter: &'a Chapter,
    pub chapter_index: usize,
    pub elapsed: f32,
    pub caption: Option<&'a str>,
}

impl Timeline {
    pub fn bundled() -> Self {
        serde_json::from_str(BUNDLED_STORY).expect("The bundled story is valid JSON")
    }

    pub fn load(directory: &Path, scenes: &[Scene]) -> Result<Self, String> {
        let path = directory.join("story.json");
        let story = match fs::read_to_string(&path) {
            Ok(text) => {
                serde_json::from_str(&text).map_err(|error| format!("story.json: {error}"))?
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Self::bundled(),
            Err(error) => return Err(format!("story.json: {error}")),
        };
        story.validate(scenes)?;
        Ok(story)
    }

    pub fn validate(&self, scenes: &[Scene]) -> Result<(), String> {
        let total: u64 = self
            .chapters
            .iter()
            .map(|chapter| u64::from(chapter.seconds))
            .sum();
        if !(1..=86400).contains(&total) {
            return Err("The story must run between 1 second and 24 hours.".into());
        }
        for chapter in &self.chapters {
            if chapter.seconds == 0 {
                return Err(format!(
                    "Chapter '{}' needs a positive duration.",
                    chapter.title
                ));
            }
            if !scenes.iter().any(|scene| scene.id == chapter.scene) {
                return Err(format!(
                    "Chapter '{}' references missing scene '{}'.",
                    chapter.title, chapter.scene
                ));
            }
            if chapter
                .light
                .is_some_and(|range| range.iter().any(|v| !(0.0..=1.0).contains(v)))
            {
                return Err(format!(
                    "Chapter '{}' light values must be between 0 and 1.",
                    chapter.title
                ));
            }
            for camera in chapter.cues.iter().filter_map(|cue| cue.camera.as_ref()) {
                for [x, y, zoom] in [camera.from, camera.to] {
                    if x.abs() > 1.0 || y.abs() > 0.5 || !(1.0..=4.0).contains(&zoom) {
                        return Err(format!(
                            "Chapter '{}' has a camera outside its framing limits.",
                            chapter.title
                        ));
                    }
                }
            }
            if chapter.cues.iter().any(|cue| cue.at >= chapter.seconds)
                || chapter.cues.windows(2).any(|pair| pair[0].at >= pair[1].at)
            {
                return Err(format!(
                    "Chapter '{}' cue times must be in order and inside the chapter.",
                    chapter.title
                ));
            }
        }
        Ok(())
    }

    pub fn duration(&self) -> f32 {
        self.start_of(self.chapters.len())
    }

    pub fn start_of(&self, index: usize) -> f32 {
        self.chapters
            .iter()
            .take(index)
            .map(|chapter| chapter.seconds)
            .sum::<u32>() as f32
    }

    pub fn adjacent_shot(&self, seconds: f64, forward: bool) -> Option<f64> {
        let mut previous = None;
        let mut chapter_start = 0;
        for chapter in &self.chapters {
            let starts =
                std::iter::once(0).chain(chapter.cues.iter().map(|cue| cue.at).filter(|&t| t > 0));
            for offset in starts {
                let shot = f64::from(chapter_start + offset);
                if forward && shot > seconds {
                    return Some(shot);
                }
                if shot < seconds {
                    previous = Some(shot);
                }
            }
            chapter_start += chapter.seconds;
        }
        if forward { None } else { previous }
    }

    pub fn moment(&self, seconds: f32) -> Moment<'_> {
        let time = seconds.clamp(0.0, self.duration());
        let mut start = 0.0;
        for (chapter_index, chapter) in self.chapters.iter().enumerate() {
            let end = start + chapter.seconds as f32;
            if time < end || chapter_index + 1 == self.chapters.len() {
                let elapsed = time - start;
                let caption = chapter
                    .cues
                    .iter()
                    .rev()
                    .find(|cue| cue.at as f32 <= elapsed)
                    .map(|cue| cue.text.as_str());
                return Moment {
                    chapter,
                    chapter_index,
                    elapsed,
                    caption,
                };
            }
            start = end;
        }
        unreachable!("Validated timelines have at least one chapter")
    }

    pub fn frame(&self, seconds: f32, scenes: &[Scene], effects: Effects) -> (usize, RenderInput) {
        let moment = self.moment(seconds);
        let index = scenes
            .iter()
            .position(|s| s.id == moment.chapter.scene)
            .expect("The timeline is validated against the loaded scenes");
        let mut settings = scenes[index].defaults;
        if let Some(seed) = moment.chapter.seed {
            settings.seed = seed;
        }
        let duration = moment.chapter.seconds as f32;
        let progress = moment.elapsed / duration;
        if let Some([start, end]) = moment.chapter.light {
            let eased = progress * progress * (3.0 - 2.0 * progress);
            settings.light = start + (end - start) * eased;
        }
        let transition = if moment.chapter_index == 0 {
            1.0
        } else {
            (moment.elapsed / 1.4).clamp(0.0, 1.0)
        };
        let transition = if moment.chapter_index + 1 == self.chapters.len() {
            transition
        } else {
            transition.min(((duration - moment.elapsed) / 1.4).clamp(0.0, 1.0))
        };
        (
            index,
            RenderInput {
                settings,
                time: moment.elapsed,
                effects,
                shot: Shot {
                    action: moment.chapter.action as u32,
                    progress,
                    camera_frame: moment.chapter.camera_at(moment.elapsed),
                    transition: 0.47 + transition * 0.53,
                },
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::Library;

    #[test]
    fn bundled_story_times_and_scene_links() {
        let scenes = Library::bundled();
        let story = Timeline::bundled();
        story.validate(&scenes).unwrap();
        assert_eq!(story.duration(), 1800.0);
        assert_eq!(story.chapters.len(), 8);
        assert_eq!(story.moment(209.99).chapter_index, 0);
        assert_eq!(story.moment(210.0).chapter_index, 1);
        assert_eq!(story.moment(210.0).elapsed, 0.0);
        assert_eq!(story.moment(1800.0).chapter_index, 7);
        assert_eq!(story.moment(1800.0).elapsed, 270.0);
        assert_eq!(
            story
                .frame(1800.0, &scenes, Effects::default())
                .1
                .shot
                .progress,
            1.0
        );
    }

    #[test]
    fn cuts_and_camera_interpolation_are_repeatable() {
        let story = Timeline::bundled();
        let chapter = &story.chapters[0];
        assert_eq!(chapter.camera_at(0.0), [0.0, 0.0, 1.0]);
        assert_eq!(chapter.camera_at(50.0), [-0.31, -0.25, 2.0]);
        assert!((chapter.camera_at(25.0)[2] - 1.115).abs() < 0.00001);
        assert_eq!(story.adjacent_shot(135.0, true), Some(210.0));
        assert_eq!(story.adjacent_shot(210.0, false), Some(135.0));
        assert_eq!(story.adjacent_shot(1800.0, true), None);
        assert_eq!(story.adjacent_shot(0.0, false), None);
        let legacy: Chapter = serde_json::from_str(
            r#"{"title":"Old","scene":"alpine-lake","seconds":8,"cues":[{"at":1,"text":"Sky"}]}"#,
        )
        .unwrap();
        assert_eq!(legacy.camera_at(4.0), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn invalid_scene_references_are_reported() {
        let mut story = Timeline::bundled();
        story.chapters[1].scene = "missing".into();
        assert!(
            story
                .validate(&Library::bundled())
                .unwrap_err()
                .contains("missing scene")
        );
    }
}
