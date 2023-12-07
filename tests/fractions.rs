use cooklang::{convert::System, Converter, Quantity, Value};
use test_case::test_case;

#[test_case(2.0, "tsp" => "2 tsp")]
#[test_case(3.0, "tsp" => "1 tbsp")]
#[test_case(3.5, "tsp" => "3 1/2 tsp")]
#[test_case(15.0, "tsp" => "5 tbsp")]
#[test_case(16.0, "tsp" => "1/3 c")]
#[test_case(180.0, "C" => "356 °F")]
#[test_case(499.999, "lb" => "500 lb")]
#[test_case(1.5, "F" => "1.5 °F")]
fn imperial(value: f64, unit: &str) -> String {
    let converter = Converter::bundled();
    let mut q = Quantity::new(Value::from(value), Some(unit.to_string()));
    let _ = q.convert(System::Imperial, &converter);
    q.to_string()
}
