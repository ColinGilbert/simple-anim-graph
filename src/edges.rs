use std::{cell::RefCell, rc::Rc};

use ozz_animation_rs::{BlendingJob, BlendingLayer, Skeleton, SoaTransform};

safe_index::new! {
TransitionIndex,
map: TransitionsContainer
}

pub struct Transition {
    pub duration: web_time::Duration,
    pub seek: web_time::Duration,
    pub blend_job: BlendingJob,
    pub output: Rc<RefCell<Vec<SoaTransform>>>,
    pub started: bool,
}

impl Transition {
    pub fn new(
        skeleton: Rc<Skeleton>,
        duration: web_time::Duration,
        from_output: Rc<RefCell<Vec<SoaTransform>>>,
        to_output: Rc<RefCell<Vec<SoaTransform>>>,
    ) -> Transition {
        let mut blend_job = BlendingJob::default();
        blend_job
            .layers_mut()
            .push(BlendingLayer::new(from_output.clone()));
        blend_job
            .layers_mut()
            .push(BlendingLayer::new(to_output.clone()));

        let output = Rc::new(RefCell::new(vec![
            SoaTransform::default();
            skeleton.num_soa_joints()
        ]));
        blend_job.set_output(output.clone());
        Transition {
            duration,
            seek: web_time::Duration::from_nanos(0),
            blend_job,
            output: output.clone(),
            started: false,
        }
    }
}
