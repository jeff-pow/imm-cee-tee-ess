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

    //trainer.load_from_checkpoint("/home/jeff/imm-cee-tee-ess/trainer/checkpoints/threats-150/");

    let schedule = TrainingSchedule {
        net_id: "threats".to_string(),
        eval_scale: 400.0,
        steps: TrainingSteps {
            batch_size: 16_384,
            batches_per_superbatch: 6104,
            start_superbatch: 0,
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

    for fen in [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    ] {
        let eval = trainer.eval(fen);
        println!("FEN: {fen}");
        println!("EVAL: {}", 400.0 * eval);
    }
}
