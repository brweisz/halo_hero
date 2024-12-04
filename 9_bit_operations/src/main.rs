use std::convert::TryInto;
use std::iter::{Iterator};
use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    dev::MockProver,
    plonk::{self, Circuit, ConstraintSystem},
};

use ff::{Field, PrimeField};
use halo2_proofs::circuit::{Value};
use halo2_proofs::plonk::{Advice, Column, Expression, Selector, TableColumn};
use halo2_proofs::poly::Rotation;

struct TestCircuit<F: Field> {
    _ph: PhantomData<F>,
}

const TABLE_OF_BIT_OPERATIONS: [[u8; 4]; 12] = [
    // XOR
    [0,0,0,0],
    [0,0,1,1],
    [0,1,0,1],
    [0,1,1,0],
    // AND
    [1,0,0,0],
    [1,0,1,0],
    [1,1,0,0],
    [1,1,1,1],
    // OR
    [2,0,0,0],
    [2,0,1,1],
    [2,1,0,1],
    [2,1,1,1],
];

#[derive(Clone, Debug)]
struct U8Chip<F: Field + Clone> {
    _ph: PhantomData<F>,
    bits: [Column<Advice>; 8],
    // t_selector: TableColumn,
    // t_left: TableColumn,
    // t_right: TableColumn,
    // t_result: TableColumn,
    t_range: TableColumn,
    q_decomposed: Selector,
    // q_xor: Selector,
    // q_and: Selector,
    // q_or: Selector,
}

#[derive(Clone, Debug)]
struct TestConfig<F: Field + Clone> {
    _ph: PhantomData<F>,
    advice: Column<Advice>,
    u8_chip: U8Chip<F>,
}

impl<F: PrimeField> U8Chip<F> {
    fn new_for(meta: &mut ConstraintSystem<F>, advice: Column<Advice>) -> Self {
        let bits = [meta.advice_column(), meta.advice_column(),
            meta.advice_column(), meta.advice_column(), meta.advice_column(),
            meta.advice_column(), meta.advice_column(), meta.advice_column()];
        let t_range = meta.lookup_table_column();
        let q_decomposed = meta.complex_selector();

        meta.lookup("Range check u8", |meta|{
            let advice_value = meta.query_advice(advice, Rotation::cur());
            let q_decomposed_ = meta.query_selector(q_decomposed);
           vec![(q_decomposed_ * advice_value, t_range)]
        });

        meta.create_gate("u8 decomposed", |meta|{
            let bits_: Vec<Expression<F>> = bits.into_iter().map(|column|{
                meta.query_advice(column, Rotation::cur())
            }).collect();

            let advice_value = meta.query_advice(advice, Rotation::cur());
            let q_decomposed = meta.query_selector(q_decomposed);
            vec![
                q_decomposed.clone() * bits_[0].clone() * (bits_[0].clone() - Expression::Constant(F::ONE)),
                q_decomposed.clone() * (bits_[1].clone() * (bits_[1].clone() - Expression::Constant(F::ONE))),
                q_decomposed.clone() * (bits_[2].clone() * (bits_[2].clone() - Expression::Constant(F::ONE))),
                q_decomposed.clone() * (bits_[3].clone() * (bits_[3].clone() - Expression::Constant(F::ONE))),
                q_decomposed.clone() * (bits_[4].clone() * (bits_[4].clone() - Expression::Constant(F::ONE))),
                q_decomposed.clone() * (bits_[5].clone() * (bits_[5].clone() - Expression::Constant(F::ONE))),
                q_decomposed.clone() * (bits_[6].clone() * (bits_[6].clone() - Expression::Constant(F::ONE))),
                q_decomposed.clone() * (bits_[7].clone() * (bits_[7].clone() - Expression::Constant(F::ONE))),
                q_decomposed.clone() * (advice_value -
                    bits_[0].clone() * Expression::Constant(F::from(1<<0)) -
                    bits_[1].clone() * Expression::Constant(F::from(1<<1)) -
                    bits_[2].clone() * Expression::Constant(F::from(1<<2)) -
                    bits_[3].clone() * Expression::Constant(F::from(1<<3)) -
                    bits_[4].clone() * Expression::Constant(F::from(1<<4)) -
                    bits_[5].clone() * Expression::Constant(F::from(1<<5)) -
                    bits_[6].clone() * Expression::Constant(F::from(1<<6)) -
                    bits_[7].clone() * Expression::Constant(F::from(1<<7))
                )
            ]
        });
        Self { _ph: PhantomData, bits, t_range, q_decomposed }
    }
}

impl<F: Field + PrimeField> TestCircuit<F>{
    fn set_lookup_table_u8(&self, layouter: &mut impl Layouter<F>, config: &TestConfig<F>){
        let _ = layouter.assign_table(|| "Range Check u8", |mut table| {
            for i in 0..256u128 {
                table.assign_cell(|| "Range check u8 table", config.u8_chip.t_range, i as usize, ||Value::known(F::from_u128(i)))?;
            }
            Ok(())
        });
    }
}

impl<F: Field + PrimeField> Circuit<F> for TestCircuit<F> {
    type Config = TestConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        TestCircuit { _ph: PhantomData }
    }

    #[allow(unused_variables)]
    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = meta.advice_column();
        let u8_chip = U8Chip::new_for(meta, advice.clone());
        TestConfig {
            _ph: PhantomData,
            advice,
            u8_chip
        }
    }

    #[allow(unused_variables)]
    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), plonk::Error> {
        self.set_lookup_table_u8(&mut layouter, &config);

        let _ = layouter.assign_region(||"Pruebita", |mut region| {
            config.u8_chip.q_decomposed.enable(&mut region, 0)?;
            region.assign_advice(||"Valor de prueba", config.advice, 0, || Value::known(F::from(7)))?;
            region.assign_advice(||"Descomposicion en bits", config.u8_chip.bits[0], 0, || Value::known(F::from(1)))?;
            region.assign_advice(||"Descomposicion en bits", config.u8_chip.bits[1], 0, || Value::known(F::from(1)))?;
            region.assign_advice(||"Descomposicion en bits", config.u8_chip.bits[2], 0, || Value::known(F::from(1)))?;
            for i in 3..8 {
                region.assign_advice(||"Descomposicion en bits", config.u8_chip.bits[i], 0, || Value::known(F::from(0)))?;
            }
            Ok(())
        });


        Ok(())
    }
}

fn main() {
    use halo2_proofs::halo2curves::bn256::Fr;
    let circuit = TestCircuit::<Fr> { _ph: PhantomData };
    let prover = MockProver::run(16, &circuit, vec![]).unwrap();
    prover.verify().unwrap();
}

#[cfg(test)]
mod test {
    #[test]
    fn test(){
        use halo2_proofs::halo2curves::bn256::Fr;
        use super::*;
        let circuit = TestCircuit::<Fr> { _ph: PhantomData };
        let prover = MockProver::run(16, &circuit, vec![]).unwrap();
        prover.verify().unwrap();
    }
}