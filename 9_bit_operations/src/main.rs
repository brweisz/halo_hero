use std::convert::TryInto;
use std::iter::{Iterator};
use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    dev::MockProver,
    plonk::{self, Circuit, ConstraintSystem},
};

use ff::{Field, PrimeField};
use halo2_proofs::circuit::{Region, Value};
use halo2_proofs::plonk::{Advice, Column, Expression, Selector, TableColumn};
use halo2_proofs::poly::Rotation;

#[derive(Copy, Clone, Debug)]
struct ExampleRow<F> {
    advice: Value<F>,
    bits: [Value<F>; 8]
}

struct TestCircuit<F: Field> {
    _ph: PhantomData<F>,
    rows: [ExampleRow<F>; 3]
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
    t_selector: TableColumn,
    t_left: TableColumn,
    t_right: TableColumn,
    t_result: TableColumn,
    t_range: TableColumn,
    q_decomposed: Selector, // TODO: separate into q_range and q_decompose
    q_xor: Selector,
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

        let t_selector = meta.lookup_table_column();
        let t_left = meta.lookup_table_column();
        let t_right = meta.lookup_table_column();
        let t_result = meta.lookup_table_column();
        let q_xor = meta.complex_selector();

        meta.create_gate("Bit xor", |meta|{
            let bits_left: Vec<Expression<F>> = bits.into_iter().map(|column|{
                meta.query_advice(column, Rotation(0)) }).collect();
            let bits_right: Vec<Expression<F>> = bits.into_iter().map(|column|{
                meta.query_advice(column, Rotation(1)) }).collect();
            let bits_result: Vec<Expression<F>> = bits.into_iter().map(|column|{
                meta.query_advice(column, Rotation(2)) }).collect();
            let q_xor = meta.query_selector(q_xor);

            let mut restrictions = vec![];
            for i in 0..8 {
                // restrictions.push((q_xor.clone() * Expression::Constant(F::ZERO), t_selector));
                // restrictions.push((q_xor.clone() * bits_left[i].clone(), t_left));
                // restrictions.push((q_xor.clone() * bits_right[i].clone(), t_right));
                // restrictions.push((q_xor.clone() * bits_result[i].clone(), t_result));

                // ------------------------------------------------------------------------

                restrictions.push(q_xor.clone() * (
                    bits_left[i].clone() * bits_left[i].clone() +
                    bits_right[i].clone() * bits_right[i].clone() -
                        Expression::Constant(F::from(2)) * bits_left[i].clone() * bits_right[i].clone() -
                        bits_result[i].clone()
                ));
            };
            restrictions
        });

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

            let mut restrictions: Vec<Expression<F>> = (0..8).into_iter().map(|i|{
                q_decomposed.clone() * bits_[i].clone() * (bits_[i].clone() - Expression::Constant(F::ONE))
            }).collect();
            restrictions.push(
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
            );
            restrictions

        });
        Self {
            _ph: PhantomData, bits, t_range, q_decomposed,
            q_xor, t_left, t_right, t_selector, t_result
        }
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

    fn set_lookup_table_xor(&self, layouter: &mut impl Layouter<F>, config: &TestConfig<F>){
        let _ = layouter.assign_table(|| "bit xor table", |mut table| {
            // TODO: please refactor this hurts
            table.assign_cell(|| "xs0", config.u8_chip.t_selector, 0, ||Value::known(F::ZERO))?;
            table.assign_cell(|| "xs1", config.u8_chip.t_selector, 1, ||Value::known(F::ZERO))?;
            table.assign_cell(|| "xs2", config.u8_chip.t_selector, 2, ||Value::known(F::ZERO))?;
            table.assign_cell(|| "xs3", config.u8_chip.t_selector, 3, ||Value::known(F::ZERO))?;

            table.assign_cell(|| "xl0", config.u8_chip.t_left, 0, ||Value::known(F::ZERO))?;
            table.assign_cell(|| "xl1", config.u8_chip.t_left, 1, ||Value::known(F::ZERO))?;
            table.assign_cell(|| "xl2", config.u8_chip.t_left, 2, ||Value::known(F::ONE))?;
            table.assign_cell(|| "xl3", config.u8_chip.t_left, 3, ||Value::known(F::ONE))?;

            table.assign_cell(|| "xr0", config.u8_chip.t_right, 0, ||Value::known(F::ZERO))?;
            table.assign_cell(|| "xr1", config.u8_chip.t_right, 1, ||Value::known(F::ONE))?;
            table.assign_cell(|| "xr2", config.u8_chip.t_right, 2, ||Value::known(F::ZERO))?;
            table.assign_cell(|| "xr3", config.u8_chip.t_right, 3, ||Value::known(F::ONE))?;

            table.assign_cell(|| "xa0", config.u8_chip.t_result, 0, ||Value::known(F::ZERO))?;
            table.assign_cell(|| "xa1", config.u8_chip.t_result, 1, ||Value::known(F::ONE))?;
            table.assign_cell(|| "xa2", config.u8_chip.t_result, 2, ||Value::known(F::ONE))?;
            table.assign_cell(|| "xa3", config.u8_chip.t_result, 3, ||Value::known(F::ZERO))?;

            Ok(())
        });
    }

    fn add_decomposed_row_to_region(&self, region: &mut Region<F>,
                                    config: &TestConfig<F>, row: [ExampleRow<F>; 3], index: usize){
        let _ = config.u8_chip.q_decomposed.enable(region, 0);
        let _ = region.assign_advice(||"Valor de prueba", config.advice, index, || row[index].advice);
        for i in 0..8 {
            let _ = region.assign_advice(||"Descomposicion en bits", config.u8_chip.bits[i], index, || row[index].bits[i]);
        }
    }
}

impl<F: Field + PrimeField> Circuit<F> for TestCircuit<F> {
    type Config = TestConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        TestCircuit {
            _ph: PhantomData,
            rows: [ExampleRow { advice: Value::unknown(), bits: [Value::unknown(); 8] }; 3]
        }
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
        self.set_lookup_table_xor(&mut layouter, &config);

        let _ = layouter.assign_region(||"Pruebita xor", |mut region| {
            let _ = config.u8_chip.q_xor.enable(&mut region, 0);
            self.add_decomposed_row_to_region(&mut region, &config, self.rows, 0);
            self.add_decomposed_row_to_region(&mut region, &config, self.rows, 1);
            self.add_decomposed_row_to_region(&mut region, &config, self.rows, 2);
            Ok(())
        });
        Ok(())
    }
}

fn main() {
    use halo2_proofs::halo2curves::bn256::Fr;
    let circuit = TestCircuit::<Fr> {
        _ph: PhantomData,
        rows: [
            ExampleRow {
                advice: Value::known(Fr::from(7)),
                bits: [
                    Value::known(Fr::from(1)), Value::known(Fr::from(1)), Value::known(Fr::from(1)),
                    Value::known(Fr::from(0)), Value::known(Fr::from(0)), Value::known(Fr::from(0)),
                    Value::known(Fr::from(0)), Value::known(Fr::from(0))
                ]
            },
            ExampleRow {
                advice: Value::known(Fr::from(8)),
                bits: [
                    Value::known(Fr::from(0)), Value::known(Fr::from(0)), Value::known(Fr::from(0)),
                    Value::known(Fr::from(1)), Value::known(Fr::from(0)), Value::known(Fr::from(0)),
                    Value::known(Fr::from(0)), Value::known(Fr::from(0))
                ]
            },
            ExampleRow {
                advice: Value::known(Fr::from(15)),
                bits: [
                    Value::known(Fr::from(1)), Value::known(Fr::from(1)), Value::known(Fr::from(1)),
                    Value::known(Fr::from(1)), Value::known(Fr::from(0)), Value::known(Fr::from(0)),
                    Value::known(Fr::from(0)), Value::known(Fr::from(0))
                ]
            },

        ]

    };
    let prover = MockProver::run(16, &circuit, vec![]).unwrap();
    prover.verify().unwrap();
}

#[cfg(test)]
mod test {
    #[test]
    fn test_should_xor_and_decompose_correctly(){
        use halo2_proofs::halo2curves::bn256::Fr;
        use super::*;
        let circuit = TestCircuit::<Fr> {
            _ph: PhantomData,
            rows: [
                ExampleRow {
                    advice: Value::known(Fr::from(7)),
                    bits: [
                        Value::known(Fr::from(1)), Value::known(Fr::from(1)), Value::known(Fr::from(1)),
                        Value::known(Fr::from(0)), Value::known(Fr::from(0)), Value::known(Fr::from(0)),
                        Value::known(Fr::from(0)), Value::known(Fr::from(0))
                    ]
                },
                ExampleRow {
                    advice: Value::known(Fr::from(8)),
                    bits: [
                        Value::known(Fr::from(0)), Value::known(Fr::from(0)), Value::known(Fr::from(0)),
                        Value::known(Fr::from(1)), Value::known(Fr::from(0)), Value::known(Fr::from(0)),
                        Value::known(Fr::from(0)), Value::known(Fr::from(0))
                    ]
                },
                ExampleRow {
                    advice: Value::known(Fr::from(15)),
                    bits: [
                        Value::known(Fr::from(1)), Value::known(Fr::from(1)), Value::known(Fr::from(1)),
                        Value::known(Fr::from(1)), Value::known(Fr::from(0)), Value::known(Fr::from(0)),
                        Value::known(Fr::from(0)), Value::known(Fr::from(0))
                    ]
                },

            ]
        };
        let prover = MockProver::run(16, &circuit, vec![]).unwrap();
        prover.verify().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_should_not_xor_and_decompose_correctly(){
        use halo2_proofs::halo2curves::bn256::Fr;
        use super::*;
        let circuit = TestCircuit::<Fr> {
            _ph: PhantomData,
            rows: [
                ExampleRow {
                    advice: Value::known(Fr::from(8)),
                    bits: [
                        Value::known(Fr::from(0)), Value::known(Fr::from(0)), Value::known(Fr::from(0)),
                        Value::known(Fr::from(1)), Value::known(Fr::from(0)), Value::known(Fr::from(0)),
                        Value::known(Fr::from(0)), Value::known(Fr::from(0))
                    ]
                },
                ExampleRow {
                    advice: Value::known(Fr::from(8)),
                    bits: [
                        Value::known(Fr::from(0)), Value::known(Fr::from(0)), Value::known(Fr::from(0)),
                        Value::known(Fr::from(1)), Value::known(Fr::from(0)), Value::known(Fr::from(0)),
                        Value::known(Fr::from(0)), Value::known(Fr::from(0))
                    ]
                },
                ExampleRow {
                    advice: Value::known(Fr::from(15)),
                    bits: [
                        Value::known(Fr::from(1)), Value::known(Fr::from(1)), Value::known(Fr::from(1)),
                        Value::known(Fr::from(1)), Value::known(Fr::from(0)), Value::known(Fr::from(0)),
                        Value::known(Fr::from(0)), Value::known(Fr::from(0))
                    ]
                },

            ]
        };
        let prover = MockProver::run(16, &circuit, vec![]).unwrap();
        prover.verify().unwrap();
    }
}