// Renders the README preview: recipe source on the left, Typst output on the right.
// From the typst/ directory:
//   typst compile --root . --ppi 144 images/preview.typ preview.png
//   sips -s format jpeg -s formatOptions 85 preview.png --out images/preview.jpg

#set page(width: 26cm, height: auto, margin: 0.8cm)
#set text(size: 9pt)

#grid(
  columns: (1fr, 1.15fr),
  gutter: 0.8cm,
  block(fill: luma(245), inset: 10pt, radius: 4pt, width: 100%)[
    #set text(size: 7.5pt)
    #raw(read("../examples/Neapolitan Pizza.cook"))
  ],
  include "../examples/neapolitan-pizza.typ",
)
