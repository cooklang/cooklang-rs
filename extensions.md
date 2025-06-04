# Cooklang syntax extensions

## Modifiers
With the ingredient modifiers you can alter the behaviour of ingredients. There
are 5 modifiers:
- `@` **Recipe**. References another recipe by it's name.
  ```cooklang
  Add @@tomato sauce{200%ml}.
  ```
- `&` **Reference**. References another ingredient with the same name. If a
  quantity is given, the amount can be added. The ingredient must be defined
  before. If there are multiple definitions, use the last one.
  ```cooklang
  Add @flour{200%g} [...], then add more @&flour{300%g}.
  ```
- `-` **Hidden**. Hidden in the list, only appears inline.
  ```cooklang
  Add some @-salt.
  ```
- `?` **Optional**. Mark the ingredient as optional.
  ```cooklang
  Now you can add @?thyme.
  ```
- `+` **New**. Forces to create a new ingredient. This works with the
  [modes](#modes) extension.

This also works (except recipe) for cookware.

## Intermediate preparations
You can refer to intermediate preparations as ingredients. For example:
```cooklang
Add @flour{200%g} and @water. Mix until combined.

Let the @&(~1)dough{} rest for ~{1%hour}.
```
Here, `dough` is refering to whatever was prepared one step back.
These ingredients will not appear in the list.

There are more syntax variations:
```cooklang
@&(~1)thing{}  -- 1 step back
@&(2)thing{}   -- step number 2
@&(=2)thing{}  -- section number 2
@&(=~2)thing{} -- 2 sections back
```

Only past steps from the current section can be referenced. It can only be
combined with the optional (`?`) modifier. Text steps can't be referenced. In
relative references, text steps are ignored. Enabling this extension
automatically enables the [modifiers](#modifiers) extension.

## Component alias
Add an alias to an ingredient to display a different name.

```cooklang
@white wine|wine{}
@@tomato sauce|sauce{}     -- works with modifiers too
```

This can be useful with references. Here, the references will be displayed as
`flour` even though the ingredient it's refering is `tipo zero flour`.

```cooklang
Add the @tipo zero flour{}
Add more @&tipo zero flour|flour{}
```

This also works for cookware.

## Advanced units
Maybe confusing name. Tweaks a little bit the parsing and behaviour of units
inside quantities.

- When the value is a number or a range and the values does not start with a
number, the unit separator (`%`) can be replaced with a space.
  ```cooklang
  @water{1 L} is the same as @water{1%L}
  ```

  If disabled, `@water{1 L}` would parse as `1 L` being a text value.
- Enables extra checks:
  - Checks that units between references are compatible, so they can be added.
  - Checks that timers have a time unit.

## Modes
Add new special metadata keys that control some of the other extensions. The
special keys are between square brackets.

```cooklang
>> [special key]: value
```

- `[mode]` | `[define]`
  - `all` | `default`. This is the default mode, same as the original cooklang.
  - `ingredients` | `components`. In this mode only components can be defined,
  all regular text is omitted. Useful for writing an ingredient list manually
  at the beginning of the recipe if you want to do so.
  - `steps`. All the ingredients are references. To force a new ingredient, use
  the new (`+`) modifier.
  - `text`. All steps are [text blocks](#text-blocks)

- `duplicate`
  - `new` | `default`. When a ingredient with the same name is found, create a
  new one. This is the original cooklang behaviour.
  - `reference` | `ref`. Ingredients have implicit references when needed. So
  ingredients with the same name will be references. To force a new ingredient,
  use the new (`+`) modifier.
    ```cooklang
    >> [duplicate]: ref
    @water{1} @water{2}
    -- is the same as
    >> [duplicate]: default
    @water{1} @&water{2}
    ```

## Temperature
Find temperatures in the text, without any markers. In the future this may be
extended to any unit.

For example, the temperature here will be parsed[^2] not as text, but as an inline
quantity.
```cooklang
Preheat the #oven to 180 ºC.
```

## Range values
Recipes are not always exact. This is a little improvement that should help
comunicating that in some cases.

```cooklang
@eggs{2-4}
@tomato sauce{200-300%ml}            -- works with units
@water{1.5-2%l}                      -- with decimal numbers too
@flour{100%g} ... @&flour{200-400%g} -- the total will be 300-500 g
```

## Timer requires time
Just an extra rule that makes timers like `~name` invalid.

[^1]: This is work in progress in `cooklang` but supported here.

[^2]: Currently this is done in the analysis pass. So in the AST there is no
concept of inline quantities.

### Name with URL

Example: `Mom's Cookbook <https://moms-cookbook.url>` -> name: `Mom's Cookbook` url: `https://moms-cookbook.url/`

The interpretations of the key value will be:

- `name <valid url>` -> as `name` & `url`
- `name <invalid url>` -> as `name`
- `name` -> as `name`
- `invalid url` -> as `name`
- `<invalid url>` -> as `name`
- `valid url` -> as `url`
- `<valid url>` -> as `url`
