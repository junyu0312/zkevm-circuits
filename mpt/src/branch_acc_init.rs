use halo2::{
    circuit::Chip,
    plonk::{Advice, Column, ConstraintSystem, Expression, VirtualCells},
    poly::Rotation,
};
use pairing::arithmetic::FieldExt;
use std::marker::PhantomData;

use crate::param::HASH_WIDTH;

#[derive(Clone, Debug)]
pub(crate) struct BranchAccInitConfig {}

// BranchAccInitChip verifies the random linear combination for the branch init row.
// The rest of random linear combination is checked in branch_acc, the whole RLC is used
// to check the hash of a branch.
pub(crate) struct BranchAccInitChip<F> {
    config: BranchAccInitConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> BranchAccInitChip<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        q_enable: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        s_rlp1: Column<Advice>,
        s_rlp2: Column<Advice>,
        s_advices: [Column<Advice>; HASH_WIDTH],
        acc_s: Column<Advice>,
        acc_mult_s: Column<Advice>,
        acc_c: Column<Advice>,
        acc_mult_c: Column<Advice>,
        acc_r: F,
    ) -> BranchAccInitConfig {
        let config = BranchAccInitConfig {};

        // TODO: constraints for branch init

        // Short RLP, meta data contains two bytes: 248, 81
        // [1,0,1,0,248,81,0,248,81,0,3,0,0,0,...
        // The length of RLP stream is: 81.

        // Long RLP, meta data contains three bytes: 249, 2, 17
        // [0,1,0,1,249,2,17,249,2,17,7,0,0,0,...
        // The length of RLP stream is: 2 * 256 + 17.

        meta.create_gate("branch init accumulator", |meta| {
            let q_enable = q_enable(meta);

            let mut constraints = vec![];

            // check branch accumulator S in row 0
            let branch_acc_s_cur = meta.query_advice(acc_s, Rotation::cur());
            let branch_acc_c_cur = meta.query_advice(acc_c, Rotation::cur());
            let branch_mult_s_cur =
                meta.query_advice(acc_mult_s, Rotation::cur());
            let branch_mult_c_cur =
                meta.query_advice(acc_mult_c, Rotation::cur());

            let two_rlp_bytes_s = meta.query_advice(s_rlp1, Rotation::cur());
            let three_rlp_bytes_s = meta.query_advice(s_rlp2, Rotation::cur());

            let two_rlp_bytes_c =
                meta.query_advice(s_advices[0], Rotation::cur());
            let three_rlp_bytes_c =
                meta.query_advice(s_advices[1], Rotation::cur());

            // TODO: two_rlp_bytes and three_rlp_bytes are bools for S and C
            // TODO: two_rlp_bytes + three_rlp_bytes = 1 for S and C

            let s_rlp1 = meta.query_advice(s_advices[2], Rotation::cur());
            let s_rlp2 = meta.query_advice(s_advices[3], Rotation::cur());
            let s_rlp3 = meta.query_advice(s_advices[4], Rotation::cur());

            let c_rlp1 = meta.query_advice(s_advices[5], Rotation::cur());
            let c_rlp2 = meta.query_advice(s_advices[6], Rotation::cur());
            let c_rlp3 = meta.query_advice(s_advices[7], Rotation::cur());

            let acc_s_two = s_rlp1.clone() + s_rlp2.clone() * acc_r;
            constraints.push((
                "branch accumulator S row 0",
                q_enable.clone()
                    * two_rlp_bytes_s.clone()
                    * (acc_s_two - branch_acc_s_cur.clone()),
            ));

            let mult_s_two = Expression::Constant(acc_r * acc_r);
            constraints.push((
                "branch mult S row 0",
                q_enable.clone()
                    * two_rlp_bytes_s
                    * (mult_s_two - branch_mult_s_cur.clone()),
            ));

            let acc_c_two = c_rlp1.clone() + c_rlp2.clone() * acc_r;
            constraints.push((
                "branch accumulator C row 0",
                q_enable.clone()
                    * two_rlp_bytes_c.clone()
                    * (acc_c_two - branch_acc_c_cur.clone()),
            ));

            let mult_c_two = Expression::Constant(acc_r * acc_r);
            constraints.push((
                "branch mult C row 0",
                q_enable.clone()
                    * two_rlp_bytes_c
                    * (mult_c_two - branch_mult_c_cur.clone()),
            ));

            let acc_s_three = s_rlp1 + s_rlp2 * acc_r + s_rlp3 * acc_r * acc_r;
            constraints.push((
                "branch accumulator S row 0 (3)",
                q_enable.clone()
                    * three_rlp_bytes_s.clone()
                    * (acc_s_three - branch_acc_s_cur),
            ));

            let mult_s_three = Expression::Constant(acc_r * acc_r * acc_r);
            constraints.push((
                "branch mult S row 0 (3)",
                q_enable.clone()
                    * three_rlp_bytes_s
                    * (mult_s_three - branch_mult_s_cur),
            ));

            let acc_c_three = c_rlp1 + c_rlp2 * acc_r + c_rlp3 * acc_r * acc_r;
            constraints.push((
                "branch accumulator C row 0 (3)",
                q_enable.clone()
                    * three_rlp_bytes_c.clone()
                    * (acc_c_three - branch_acc_c_cur),
            ));

            let mult_c_three = Expression::Constant(acc_r * acc_r * acc_r);
            constraints.push((
                "branch mult C row 0 (3)",
                q_enable
                    * three_rlp_bytes_c
                    * (mult_c_three - branch_mult_c_cur),
            ));

            constraints
        });

        config
    }

    pub fn construct(config: BranchAccInitConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }
}

impl<F: FieldExt> Chip<F> for BranchAccInitChip<F> {
    type Config = BranchAccInitConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}
