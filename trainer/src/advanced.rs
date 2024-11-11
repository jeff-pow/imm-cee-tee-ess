use crate::threat_inputs::ThreatInput;
use bullet::{loader, lr, optimiser, wdl, LocalSettings, TrainingSchedule, TrainingSteps};
use bullet::{
    operations::{self, sparse_affine_dual_with_activation},
    optimiser::AdamWOptimiser,
    outputs, Activation, ExecutionContext, Graph, GraphBuilder, Node, QuantTarget, Shape, Trainer,
};

use imm_cee_tee_ess::eval::{INPUT_SIZE, L1_SIZE};

pub fn train() {
    let (mut graph, output_node) = build_network();

    seed_weights(&mut graph);

    let mut trainer = Trainer::<AdamWOptimiser, ThreatInput>::new(
        graph,
        output_node,
        optimiser::AdamWParams {
            decay: 0.01,
            beta1: 0.9,
            beta2: 0.999,
            min_weight: -1.98,
            max_weight: 1.98,
        },
        ThreatInput,
        outputs::Single,
        vec![
            ("ftw".to_string(), QuantTarget::Float),
            ("ftb".to_string(), QuantTarget::Float),
            ("l1w".to_string(), QuantTarget::Float),
            ("l1b".to_string(), QuantTarget::Float),
            ("l2w".to_string(), QuantTarget::Float),
            ("l2b".to_string(), QuantTarget::Float),
            ("l3w".to_string(), QuantTarget::Float),
            ("l3b".to_string(), QuantTarget::Float),
        ],
    );

    let schedule = TrainingSchedule {
        net_id: "threats".to_string(),
        eval_scale: 400.0,
        steps: TrainingSteps {
            batch_size: 16_384,
            batches_per_superbatch: 6104,
            start_superbatch: 0,
            end_superbatch: 750,
        },
        wdl_scheduler: wdl::ConstantWDL { value: 0.75 },
        lr_scheduler: lr::ExponentialDecayLR {
            initial_lr: 1e-3,
            final_lr: 1e-7,
            final_superbatch: 750,
        },
        save_rate: 10,
    };

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
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R b KQkq - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 b kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R b KQ - 1 8",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 b - - 0 1",
    ] {
        let eval = trainer.eval(fen);
        println!("FEN: {fen}");
        println!("EVAL: {}", 400.0 * eval);
    }
}

fn build_network() -> (Graph, Node) {
    let mut builder = GraphBuilder::default();

    // inputs
    let stm = builder.create_input("stm", Shape::new(INPUT_SIZE, 1));
    let nstm = builder.create_input("nstm", Shape::new(INPUT_SIZE, 1));
    let targets = builder.create_input("targets", Shape::new(1, 1));

    let ftw = builder.create_weights("ftw", Shape::new(L1_SIZE, INPUT_SIZE));
    let ftb = builder.create_weights("ftb", Shape::new(L1_SIZE, 1));

    let l1w = builder.create_weights("l1w", Shape::new(16, L1_SIZE * 2));
    let l1b = builder.create_weights("l1b", Shape::new(16, 1));

    let l2w = builder.create_weights("l2w", Shape::new(32, 16));
    let l2b = builder.create_weights("l2b", Shape::new(32, 1));

    let l3w = builder.create_weights("l3w", Shape::new(1, 32));
    let l3b = builder.create_weights("l3b", Shape::new(1, 1));

    // inference
    let ft = sparse_affine_dual_with_activation(&mut builder, ftw, stm, nstm, ftb, Activation::SCReLU);

    let l1 = operations::affine(&mut builder, l1w, ft, l1b);
    let l1 = operations::activate(&mut builder, l1, Activation::SCReLU);

    let l2 = operations::affine(&mut builder, l2w, l1, l2b);
    let l2 = operations::activate(&mut builder, l2, Activation::SCReLU);

    let predicted = operations::affine(&mut builder, l3w, l2, l3b);
    let sigmoided = operations::activate(&mut builder, predicted, Activation::Sigmoid);
    operations::mse(&mut builder, sigmoided, targets);

    // graph, output node
    (builder.build(ExecutionContext::default()), predicted)
}

fn seed_weights(graph: &mut Graph) {
    graph
        .get_weights_mut("ftw")
        .seed_random(0.0, 1.0 / (INPUT_SIZE as f32).sqrt(), true);
    graph
        .get_weights_mut("ftb")
        .seed_random(0.0, 1.0 / (INPUT_SIZE as f32).sqrt(), true);
    graph
        .get_weights_mut("l1w")
        .seed_random(0.0, 1.0 / (2. * L1_SIZE as f32).sqrt(), true);
    graph
        .get_weights_mut("l1b")
        .seed_random(0.0, 1.0 / (2. * L1_SIZE as f32).sqrt(), true);

    for name in ["l2w", "l2b", "l3w", "l3b"] {
        graph
            .get_weights_mut(name)
            .seed_random(0.0, 1.0 / (16_f32).sqrt(), true);
    }
}
