# cooklang-ingredient-parser

Turns one typed ingredient line into a name and an amount, the same way in
every Cooklang app.

    use cooklang_ingredient_parser::parse_ingredient;

    let parsed = parse_ingredient("Sugar 1.5kg", None, None);
    assert_eq!(parsed.name, "Sugar");
    assert_eq!(parsed.amount.unwrap().to_quantity().as_deref(), Some("1.5%kg"));

- The amount can be at the end or the start ("Milk 2 l", "2l milk"), in six
  languages ("Сахар 200 гр"). A number glued to a word stays in the name ("7up").
- Pass an aisle configuration to take its spelling of a name, ignoring case
  and simple English plurals ("Tomato" → "tomatoes").
- Pass an `IngredientModel` (such as `ingredient-tagger`) to read lines that
  lead with a number the rules cannot place ("2 cans chopped tomatoes").
- `parse_ingredient_list` splits pasted text, dropping bullets, checkboxes
  and Markdown headings.
