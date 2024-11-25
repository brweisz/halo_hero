fn configure(meta: &mut ConstraintSystem<F>) --> Configuración sobre el circuito. Define...
- Las columnas de la traza (selectoras, advice)
- Las Gates existentes
- Permutaciones
- Lookups
- Etc

Se hace a través de retornar un struct que representa la configuración. Internamente usa un objeto ConstraintSystem:
"This is a description of the circuit environment, such as the gate, column and permutation arrangements."

------------------------------------------------

fn synthesize(config: Self::Config, mut layouter: impl Layouter<F>) --> Toma la configuración mencionada previamente y
    usa un Layouter para guardar el estado del circuito que está siendo construido. Este a su vez se ocupa de organizar
    las regiones de la traza. Basicamente tiene que construir la traza y basarse en la forma que tiene la misma según
    la configuración.

Estructura de la traza
----------------------
```rust 
pub struct Cell {
    /// Identifies the region in which this cell resides.
    pub region_index: RegionIndex,
    /// The relative offset of this cell within its region.
    pub row_offset: usize,
    /// The column of this cell.
    pub column: Column<Any>,
}

pub struct AssignedCell<V, F: Field> {
    value: Value<V>,
    cell: Cell,
}
```