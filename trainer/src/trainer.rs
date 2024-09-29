use bullet::{
    format::{chess::BoardIter, ChessBoard},
    inputs::InputType,
    loader, lr, optimiser, outputs, wdl, Activation, LocalSettings, Loss, TrainerBuilder, TrainingSchedule,
};
use imm_cee_tee_ess::{
    board::Board,
    types::{bitboard::Bitboard, pieces::Color, square::Square},
};

pub fn go_pack() {
    let mut trainer = TrainerBuilder::default()
        .quantisations(&[255, 64])
        .optimiser(optimiser::AdamW)
        .input(ThreatInput)
        .output_buckets(outputs::Single)
        .feature_transformer(1024)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(1)
        .build();

    //trainer.load_from_checkpoint("checkpoints/testnet");

    let schedule = TrainingSchedule {
        net_id: "testnet".to_string(),
        eval_scale: 400.0,
        ft_regularisation: 0.0,
        batch_size: 16_384,
        batches_per_superbatch: 6104,
        start_superbatch: 1,
        end_superbatch: 300,
        wdl_scheduler: wdl::ConstantWDL { value: 0.0 },
        lr_scheduler: lr::StepLR {
            start: 0.001,
            gamma: 0.3,
            step: 60,
        },
        loss_function: Loss::SigmoidMSE,
        save_rate: 10,
        optimiser_settings: optimiser::AdamWParams {
            decay: 0.01,
            beta1: 0.9,
            beta2: 0.999,
            min_weight: -1.98,
            max_weight: 1.98,
        },
    };

    let settings = LocalSettings {
        threads: 4,
        test_set: None,
        output_directory: "checkpoints",
        batch_queue_size: 512,
    };

    let data_loader = loader::DirectSequentialDataLoader::new(&["../batch1.data"]);

    trainer.run(&schedule, &settings, &data_loader);
}

#[derive(Clone, Copy, Debug, Default)]
struct ThreatInput;

impl InputType for ThreatInput {
    type RequiredDataType = ChessBoard;
    type FeatureIter = ThreatIter;

    fn max_active_inputs(&self) -> usize {
        32
    }

    fn inputs(&self) -> usize {
        768 * 4
    }

    fn buckets(&self) -> usize {
        1
    }

    fn feature_iter(&self, pos: &Self::RequiredDataType) -> Self::FeatureIter {
        let mut pieces = [Bitboard::EMPTY; 6];
        let mut colors = [Bitboard::EMPTY; 2];
        for (piece, sq) in pos.into_iter() {
            let sq = Square(sq);
            let c = usize::from(piece & 8 > 0);
            let pc = usize::from(piece & 7);
            pieces[pc] |= sq.bitboard();
            colors[c] |= sq.bitboard();
        }
        // Brother I'm not sure if stm being white is right. Bullet is always stm relative
        // but I haven't puzzled through it enough.
        let mut board = Board::from_bbs(pieces, colors, Color::White);
        let threats = board.threats();
        board.stm = Color::Black;
        let defenders = board.threats();
        ThreatIter {
            board_iter: pos.into_iter(),
            threats,
            defenders,
        }
    }

    fn size(&self) -> usize {
        self.inputs() * self.buckets()
    }
}

struct ThreatIter {
    board_iter: BoardIter,
    threats: Bitboard,
    defenders: Bitboard,
}

impl Iterator for ThreatIter {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.board_iter.next().map(|(piece, square)| {
            let c = usize::from(piece & 8 > 0);
            let p = 64 * usize::from(piece & 7);
            let sq = usize::from(square);

            let map_feature = |feat, threats: Bitboard, defenders: Bitboard| {
                2 * 768 * usize::from(threats.contains(sq.into()))
                    + 768 * usize::from(defenders.contains(sq.into()))
                    + feat
            };

            let stm_feat = [0, 384][c] + p + sq;
            let xstm_feat = [384, 0][c] + p + (sq ^ 56);

            (
                map_feature(stm_feat, self.threats, self.defenders),
                map_feature(xstm_feat, self.defenders, self.threats),
            )
        })
    }
}
