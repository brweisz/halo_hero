```fn configure(meta: &mut ConstraintSystem<F>)``` --> Configuración sobre el circuito. Define...
- Las columnas de la traza (selectoras, advice, fixed)
- Las Gates existentes
- Permutaciones
- Lookups
- Etc

Se retorna un struct que representa la configuración. Internamente usa un objeto ConstraintSystem:
"This is a description of the circuit environment, such as the gate, column and permutation arrangements."

Acá se usan varios otros objetos:
* Selector: sirve para activar o desactivar constraints sobre las columnas.
* Column<ColumnType>
  * ColumnType: hay varios pero el que vi hasta ahora es Advice

------------------------------------------------

```fn synthesize(config: Self::Config, mut layouter: impl Layouter<F>)``` --> Toma la configuración mencionada previamente y
    usa un Layouter para guardar el estado del circuito que está siendo construido. Este a su vez se ocupa de crear y 
    organizar las regiones de la traza. Basicamente tiene que construir la traza y basarse en la forma que tiene la 
    misma según la configuración.

layouter.assign_region(nombre de la region, funcion que recibe la region y la configura) --> Devuelve una AssignedCell
layouter.assign_table(nombre de la tabla, funcion que recibe la tabla y la configura) --> Result<(), Error>
layouter.constrain_instance(una celda cualquiera, columna instance, fila en la columna instance)?;

La región tiene los métodos:

region.assign_advice(descripcion, 
                     instancia de columna (de tipo advice), 
                     offset dentro de la region,
                     to: valor)

region.assign_fixed(descripcion,
                    column,
                    offset in region,
                    to: valor)

region.constrain_equal(cell, cell)

La table tiene los metodos:

table.assign_cell( || Nombre tabla,
                    columna de tabla correspondiente,
                    offset,
                    || Value::known(x))?;

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

