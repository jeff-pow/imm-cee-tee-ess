use crate::threat_inputs::ThreatInput;
use bullet::{
    loader, lr, optimiser, outputs, wdl, Activation, LocalSettings, Loss, NetworkTrainer, TrainerBuilder,
    TrainingSchedule, TrainingSteps,
};

pub fn train() {
    let mut trainer = TrainerBuilder::default()
        .loss_fn(Loss::SigmoidMSE)
        .optimiser(optimiser::AdamW)
        .input(ThreatInput)
        .output_buckets(outputs::Single)
        .feature_transformer(768)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(1)
        .build();

    trainer.load_from_checkpoint("/home/jeff/imm-cee-tee-ess/trainer/checkpoints/threats-150/");

    let schedule = TrainingSchedule {
        net_id: "threats".to_string(),
        eval_scale: 400.0,
        steps: TrainingSteps {
            batch_size: 16_384,
            batches_per_superbatch: 6104,
            start_superbatch: 150,
            end_superbatch: 250,
        },
        wdl_scheduler: wdl::ConstantWDL { value: 0.0 },
        lr_scheduler: lr::StepLR {
            start: 0.001,
            gamma: 0.3,
            step: 50,
        },
        save_rate: 10,
    };

    let optimiser_params = optimiser::AdamWParams {
        decay: 0.01,
        beta1: 0.9,
        beta2: 0.999,
        min_weight: -1.98,
        max_weight: 1.98,
    };

    trainer.set_optimiser_params(optimiser_params);

    let settings = LocalSettings {
        threads: 4,
        test_set: None,
        output_directory: "checkpoints",
        batch_queue_size: 512,
    };

    let data_loader = loader::DirectSequentialDataLoader::new(&[
        "/home/jeff/chess-data/shuffled-test80-jan2023-16tb7p-filter-v6-sk20.min-mar2023.bin",
    ]);

    trainer.run(&schedule, &settings, &data_loader);
}
