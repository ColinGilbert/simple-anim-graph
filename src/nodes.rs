use ozz_animation_rs::*;
use std::cell::RefCell;
use std::rc::Rc;

pub enum GenericNode {
    Sampler(SamplerNode),
    BlendTreeOneDim(BlendTreeOneDimNode)
}
pub struct SamplerNode {
    pub output: Rc<RefCell<Vec<SoaTransform>>>,
    pub speed: f32,
    sample_job: ozz_animation_rs::SamplingJobRc,
    seek: f32,
    looping: bool,
    finished: bool,
}

impl SamplerNode {
    pub fn new(skeleton: Rc<Skeleton>, animation: Rc<Animation>, looping: bool) -> Self {
        let mut sample_job = ozz_animation_rs::SamplingJob::default();
        sample_job.set_animation(animation.clone());

        sample_job.set_context(SamplingContext::new(animation.num_tracks()));

        let output = Rc::new(RefCell::new(vec![
            SoaTransform::default();
            skeleton.num_soa_joints()
        ]));

        sample_job.set_output(output.clone());

        SamplerNode {
            output,
            speed: 1.0,
            sample_job,
            seek: 0.0,
            looping,
            finished: false,
        }
    }

    pub fn update(&mut self, dt: web_time::Duration) {
        let duration = self.sample_job.animation().unwrap().duration();
        self.seek += dt.as_secs_f32() * self.speed;
        if self.looping && !self.finished {
            self.seek %= duration;
        } else {
            if self.seek > duration {
                self.seek = 0.0;
                self.finished = true;
            }
        }
        let ratio = self.seek / duration;
        self.sample_job.set_ratio(ratio);
        self.sample_job.run().unwrap();
    }

    pub fn reset(&mut self) {
        self.finished = false;
        self.seek = 0.0;
        self.speed = 1.0;
    }
}

pub struct BlendTreeOneDimNode {
    pub output: Rc<RefCell<Vec<SoaTransform>>>,
    pub playback_speed: f32,
    pub param: f32,
    blend_job: BlendingJobRc,
    sample_jobs: Vec<SamplingJobRc>,
}

impl BlendTreeOneDimNode {
    pub fn new(skeleton: Rc<Skeleton>, anims: Vec<Rc<Animation>>) -> Self {
        let mut sample_jobs = Vec::<SamplingJobRc>::new();
        let mut blend_job = BlendingJobRc::default();
        blend_job.set_skeleton(skeleton.clone());

        for a in anims {
            let mut sample = SamplingJobRc::default();
            sample.set_animation(a.clone());
            sample.set_context(SamplingContext::new(a.num_tracks()));
            let sample_out = Rc::new(RefCell::new(vec![SoaTransform::default(); skeleton.num_soa_joints()]));
            sample.set_output(sample_out.clone());
            sample_jobs.push(sample);
            blend_job.layers_mut().push(BlendingLayer::new(sample_out.clone()));
            let layers_idx = blend_job.layers().len() - 1;
            blend_job.layers_mut()[layers_idx].weight = 0.0;
        }

        let output = blend_job.output().unwrap();

        BlendTreeOneDimNode {
            output: output.clone(),
            playback_speed: 1.0,
            param: 0.0,
            blend_job,
            sample_jobs,
        }
    }

    pub fn update(&mut self, dt: web_time::Duration) {

    }
}
