//! cargo run --example spec2_check -- recipes/*.cook
//! Parses each file with every extension and prints a one-line summary.
use cooklang::{Content, Converter, CooklangParser, Extensions, Item};
fn main() {
    let p = CooklangParser::new(Extensions::all(), Converter::bundled());
    for path in std::env::args().skip(1) {
        let input = std::fs::read_to_string(&path).unwrap();
        let name = std::path::Path::new(&path)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let res = p.parse(&input);
        let errs: Vec<String> = res
            .report()
            .errors()
            .map(|e| e.message.to_string())
            .collect();
        let warns: Vec<String> = res
            .report()
            .warnings()
            .map(|e| e.message.to_string())
            .collect();
        let Some(r) = res.output() else {
            println!("{name:32} NO OUTPUT {errs:?}");
            continue;
        };
        let (mut steps, mut actions, mut groups, mut labels, mut arrows) = (0, 0, 0, 0, 0);
        for s in &r.sections {
            for c in &s.content {
                if let Content::Step(st) = c {
                    steps += 1;
                    labels += st.label.is_some() as usize;
                    for i in &st.items {
                        match i {
                            Item::Action { .. } => actions += 1,
                            Item::Group { .. } => groups += 1,
                            Item::Text { value } if value.contains(" ->") => arrows += 1,
                            _ => {}
                        }
                    }
                }
            }
        }
        let uses = r
            .ingredients
            .iter()
            .filter(|i| i.relation.is_product_reference())
            .count();
        println!("{name:32} steps={steps:2} products={:2} uses={uses:2} actions={actions:2} groups={groups} step_labels={labels} sec_labels={} arrows_left={arrows} errors={errs:?} warnings={:?}",
            r.products.len(), r.sections.iter().filter(|s| s.label.is_some()).count(), warns);
    }
}
