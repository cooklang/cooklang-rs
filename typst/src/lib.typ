#let p = plugin("../cooklang-core/cooklang_core.wasm")

#let parse(content) = {
  // Pass cooklang recipe content to the parser and return the resulting JSON
  // todo: do you have to check if content is bytes or string?
  json(p.parse(bytes(content)))
}