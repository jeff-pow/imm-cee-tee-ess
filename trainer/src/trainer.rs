use crate::threat_inputs::ThreatInput;
use bullet::{
    format::{chess::BoardIter, ChessBoard},
    inputs::InputType,
    loader, lr, optimiser, outputs, wdl, Activation, LocalSettings, Loss, TrainerBuilder, TrainingSchedule,
};
use imm_cee_tee_ess::{
    board::Board,
    types::{bitboard::Bitboard, pieces::Color, square::Square},
};
use std::mem::TransmuteFrom;

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

    //trainer.load_from_checkpoint("checkpoints/testnet");

    let schedule = TrainingSchedule {
        net_id: "simple".to_string(),
        eval_scale: 400.0,
        steps: TrainingSteps {
            batch_size: 16_384,
            batches_per_superbatch: 6104,
            start_superbatch: 1,
            end_superbatch: 20,
        },
        wdl_scheduler: wdl::ConstantWDL { value: 0.0 },
        lr_scheduler: lr::StepLR {
            start: 0.001,
            gamma: 0.1,
            step: 8,
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

    let data_loader = loader::DirectSequentialDataLoader::new(&["data/monty-1000m.data"]);

    trainer.run(&schedule, &settings, &data_loader);
}
