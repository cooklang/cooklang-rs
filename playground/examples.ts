export interface Example {
    id: string;
    label: string;
    source: string;
}

export const examples: Example[] = [
    {
        id: "pancakes",
        label: "Pancakes — the basics",
        source: `---
title: Pancakes
servings: 2
time: 25 min
---
Crack the @eggs{3} into a #blender, add the @plain flour{125%g}, @milk{250%ml} and @sea salt{1%pinch}, and blitz until smooth.

Pour into a #bowl and leave to stand for ~{15%minutes}.

Melt the @butter{1%knob} in a #frying pan{} and cook each pancake for ~{2%minutes} a side.
`,
    },
    {
        id: "pizza",
        label: "Pizza — sections & comments",
        source: `---
title: Neapolitan Pizza
servings: 2
time: 45 min
tags: [italian, pizza]
source: https://cooklang.org
---
== Dough ==
Mix @tipo 00 flour{500%g}, @water{325%ml}, @salt{10%g} and @fresh yeast{2%g} in a #large bowl{}.

Knead for ~{10%minutes}, then cover and leave to rise for ~{8%hours}.

== Pizza ==
-- The oven takes a while, start it early
Preheat the #oven to 250°C.

Stretch the dough and spread over @San Marzano tomatoes{200%g}. Add @mozzarella{125%g} and a few @basil leaves{}.

Bake for ~{8%minutes} until the crust is blistered.
`,
    },
    {
        id: "chili",
        label: "Chili — extensions",
        source: `---
title: Weeknight Chili
servings: 4
tags: [dinner, freezable]
---
> Notes, preparations, ranges, optional ingredients and aliases are extensions to the base spec.

Fry the @onion{1}(diced) in @olive oil{2%tbsp} in a #heavy pot{}.

Add @ground beef{500%g} and @chili powder{2-3%tsp}, then @?jalapeño{1}(sliced) if you like it hot.

Pour in @chopped tomatoes{2%tins} and simmer for ~{25-30%minutes}.

[- Block comments are ignored by every app -]
Serve topped with @green onion|scallions{2}(sliced).
`,
    },
];

export const defaultExample = examples[0];
