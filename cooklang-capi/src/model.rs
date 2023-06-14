use std::{
    ffi::{c_char, CStr, CString},
    ptr,
};

use crate::cstring_new;

pub struct Recipe {
    inner: InnerRecipe,
}

enum InnerRecipe {
    Scaled(cooklang::ScaledRecipe),
    NotScaled(cooklang::Recipe),
}

impl Recipe {
    pub(crate) fn new(recipe: cooklang::Recipe) -> Self {
        Self {
            inner: InnerRecipe::NotScaled(recipe),
        }
    }
}

pub struct Metadata {
    inner: cooklang::metadata::Metadata,
    c_string: Option<CString>,
}

impl Metadata {
    pub(crate) fn new(metadata: cooklang::metadata::Metadata) -> Self {
        Self {
            inner: metadata,
            c_string: None,
        }
    }
}

/// Get a value for a key from the metadata dict.
///
/// The returned string is valid until this function is called again or
/// the metadata pointer is freed.
///
/// Returns NULL if not found.
#[no_mangle]
pub extern "C" fn cook_metadata_get(
    metadata: *const Metadata,
    key: *const c_char,
) -> *const c_char {
    let meta = unsafe { &mut *(metadata as *mut Metadata) };

    let key = unsafe { CStr::from_ptr(key) };
    let key = match key.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null(),
    };

    match meta.inner.map.get(key) {
        Some(value) => {
            meta.c_string = Some(cstring_new(value.as_str()));
            meta.c_string.as_ref().unwrap().as_ptr()
        }
        None => ptr::null(),
    }
}

pub struct Ast<'a>(pub(crate) cooklang::ast::Ast<'a>);

#[no_mangle]
pub extern "C" fn cook_recipe_scale(
    recipe: *mut *mut Recipe,
    target: u32,
    parser: *const crate::CookParser,
) {
    let parser = unsafe { &*parser };
    let wrapped = unsafe { Box::from_raw(*recipe) };
    match wrapped.inner {
        InnerRecipe::Scaled(_) => panic!("already scaled"),
        InnerRecipe::NotScaled(r) => {
            let scaled = r.scale(target, parser.0.converter());
            unsafe {
                *recipe = Box::into_raw(Box::new(Recipe {
                    inner: InnerRecipe::Scaled(scaled),
                }));
            }
        }
    };
}

#[no_mangle]
pub extern "C" fn cook_recipe_default_scale(recipe: *mut *mut Recipe) {
    let wrapped = unsafe { Box::from_raw(*recipe) };
    match wrapped.inner {
        InnerRecipe::Scaled(_) => panic!("already scaled"),
        InnerRecipe::NotScaled(r) => {
            let scaled = r.default_scale();
            unsafe {
                *recipe = Box::into_raw(Box::new(Recipe {
                    inner: InnerRecipe::Scaled(scaled),
                }));
            }
        }
    };
}

#[no_mangle]
pub extern "C" fn cook_recipe_free(recipe: *const Recipe) {
    unsafe { drop(Box::from_raw(recipe as *mut Recipe)) }
}

fn ffi_vec<S, T>(src: impl Iterator<Item = S>, func: impl FnMut(S) -> T) -> (*const T, usize) {
    let slice = src.map(func).collect::<Box<[_]>>();
    let len = slice.len();
    (Box::into_raw(slice) as *const T, len)
}

unsafe fn ffi_vec_from_raw<T>(ptr: *const T, len: usize) -> Box<[T]> {
    let slice = std::slice::from_raw_parts_mut(ptr as *mut T, len);
    let boxed_slice: Box<[T]> = Box::from_raw(slice);
    boxed_slice
}

fn opt_cstring_ptr(src: Option<&str>) -> *const c_char {
    src.as_ref()
        .map_or(ptr::null(), |&s| cstring_new(s).into_raw())
}

/// Gets the data of the inner recipe.
///
/// Once the data is extracted, the recipe obtained from the result can be freed
/// and this will be valid until `cooklang_recipe_data_free` is called.
///
/// This recipe cannot be converted or scaled. Do that before getting the data.
#[no_mangle]
pub extern "C" fn cook_recipe_data(recipe: *mut Recipe) -> *const crate::RecipeData {
    let recipe = unsafe { &mut *recipe };

    let data = match &recipe.inner {
        InnerRecipe::Scaled(r) => r.to_ffi(),
        InnerRecipe::NotScaled(r) => r.to_ffi(),
    };

    Box::into_raw(Box::new(data))
}

#[no_mangle]
pub extern "C" fn cook_recipe_data_free(recipe: *const RecipeData) {
    let mut recipe = unsafe { *Box::from_raw(recipe as *mut RecipeData) };

    recipe.ffi_free();
}

trait ToFfi {
    type Target;
    fn to_ffi(&self) -> Self::Target;
}

trait FfiFree {
    fn ffi_free(&mut self);
}

#[repr(transparent)]
pub struct CCString(*const c_char);

impl CCString {
    fn null() -> Self {
        Self(ptr::null())
    }
}

impl ToFfi for Option<&str> {
    type Target = CCString;

    fn to_ffi(&self) -> Self::Target {
        CCString(opt_cstring_ptr(*self))
    }
}

impl ToFfi for &str {
    type Target = CCString;

    fn to_ffi(&self) -> Self::Target {
        CCString(cstring_new(*self).into_raw())
    }
}

impl FfiFree for CCString {
    fn ffi_free(&mut self) {
        if !self.0.is_null() {
            unsafe { drop(CString::from_raw(self.0 as *mut c_char)) }
        }
        *self = Self::null();
    }
}

#[repr(C)]
pub struct RecipeData {
    pub name: CCString,
    pub metadata: *const Metadata,

    pub sections: *const Section,
    pub sections_len: usize,

    pub ingredients: *const Ingredient,
    pub ingredients_len: usize,

    pub cookware: *const Cookware,
    pub cookware_len: usize,

    pub timers: *const Timer,
    pub timers_len: usize,

    pub inline_quantities: *const Quantity,
    pub inline_quantities_len: usize,
}

impl<D> ToFfi for cooklang::Recipe<D> {
    type Target = RecipeData;

    fn to_ffi(&self) -> Self::Target {
        let (sections, sections_len) = ffi_vec(self.sections.iter(), ToFfi::to_ffi);
        let (ingredients, ingredients_len) = ffi_vec(self.ingredients.iter(), ToFfi::to_ffi);
        let (cookware, cookware_len) = ffi_vec(self.cookware.iter(), ToFfi::to_ffi);
        let (timers, timers_len) = ffi_vec(self.timers.iter(), ToFfi::to_ffi);
        let (inline_quantities, inline_quantities_len) =
            ffi_vec(self.inline_quantities.iter(), ToFfi::to_ffi);

        RecipeData {
            name: self.name.as_str().to_ffi(),
            metadata: Box::into_raw(Box::new(Metadata::new(self.metadata.clone()))),
            sections,
            sections_len,
            ingredients,
            ingredients_len,
            cookware,
            cookware_len,
            timers,
            timers_len,
            inline_quantities,
            inline_quantities_len,
        }
    }
}

impl FfiFree for RecipeData {
    fn ffi_free(&mut self) {
        self.name.ffi_free();

        // boxed slices will be deallocated when dropped
        let mut sections = unsafe { ffi_vec_from_raw(self.sections, self.sections_len) };
        for section in sections.iter_mut() {
            section.ffi_free();
        }
        let mut ingredients = unsafe { ffi_vec_from_raw(self.ingredients, self.ingredients_len) };
        for ingredient in ingredients.iter_mut() {
            ingredient.ffi_free();
        }
        let mut cookware = unsafe { ffi_vec_from_raw(self.cookware, self.cookware_len) };
        for c in cookware.iter_mut() {
            c.ffi_free();
        }
        let mut timers = unsafe { ffi_vec_from_raw(self.timers, self.timers_len) };
        for t in timers.iter_mut() {
            t.ffi_free();
        }
        let mut inline_quantities =
            unsafe { ffi_vec_from_raw(self.inline_quantities, self.inline_quantities_len) };
        for q in inline_quantities.iter_mut() {
            q.ffi_free();
        }
    }
}

#[repr(C)]
pub struct Section {
    pub name: CCString,
    pub steps: *const Step,
    pub steps_len: usize,
}

impl ToFfi for cooklang::model::Section {
    type Target = Section;

    fn to_ffi(&self) -> Self::Target {
        let name = self.name.as_deref().to_ffi();
        let (steps, steps_len) = ffi_vec(self.steps.iter(), |step| step.to_ffi());
        Section {
            name,
            steps,
            steps_len,
        }
    }
}

impl FfiFree for Section {
    fn ffi_free(&mut self) {
        self.name.ffi_free();
        let mut steps = unsafe { ffi_vec_from_raw(self.steps, self.steps_len) };
        for step in steps.iter_mut() {
            step.ffi_free();
        }
    }
}

#[repr(C)]
pub struct Step {
    pub items: *const Item,
    pub items_len: usize,
    pub is_text: bool,
}

impl ToFfi for cooklang::model::Step {
    type Target = Step;

    fn to_ffi(&self) -> Self::Target {
        let (items, items_len) = ffi_vec(self.items.iter(), |item| match item {
            cooklang::model::Item::Text(s) => Item::Text(cstring_new(s.as_str()).into_raw()),
            cooklang::model::Item::Component(c) => Item::Component(Component {
                kind: match c.kind {
                    cooklang::model::ComponentKind::Ingredient => ComponentKind::Ingredient,
                    cooklang::model::ComponentKind::Cookware => ComponentKind::Cookware,
                    cooklang::model::ComponentKind::Timer => ComponentKind::Cookware,
                },
                index: c.index,
            }),
            cooklang::model::Item::InlineQuantity(index) => Item::InlineQuantity(*index),
        });
        Step {
            items,
            items_len,
            is_text: self.is_text,
        }
    }
}

impl FfiFree for Step {
    fn ffi_free(&mut self) {
        let items = unsafe { ffi_vec_from_raw(self.items, self.items_len) };
        for item in items.iter() {
            match item {
                Item::Text(s) => unsafe { drop(CString::from_raw(*s as *mut c_char)) },
                Item::Component(_) => {}
                Item::InlineQuantity(_) => {}
            }
        }
    }
}

/// cbindgen:prefix-with-name
#[repr(C)]
pub enum Item {
    /// Just plain text
    Text(*const c_char),
    /// A [Component]
    Component(Component),
    /// An inline quantity.
    ///
    /// The number inside is an index into [Recipe::inline_quantities].
    InlineQuantity(usize),
}

/// A component reference
#[repr(C)]
pub struct Component {
    /// What kind of component is
    pub kind: ComponentKind,
    /// The index in the corresponding [Vec] in the [Recipe] struct.
    pub index: usize,
}

/// cbindgen:prefix-with-name
#[repr(C)]
pub enum ComponentKind {
    Ingredient,
    Cookware,
    Timer,
}

#[repr(C)]
pub struct Ingredient {
    name: CCString,
    /// nullable
    alias: CCString,
    /// nullable
    quantity: *const Quantity,
    /// nullable
    note: CCString,
    modifiers: u32,
    /// index to definition. -1 if this is the definition
    references_to: isize,
}

impl ToFfi for cooklang::model::Ingredient {
    type Target = Ingredient;

    fn to_ffi(&self) -> Self::Target {
        let name = self.name.as_str().to_ffi();
        let alias = self.alias.as_deref().to_ffi();
        let quantity = self.quantity.to_ffi();
        let note = self.note.as_deref().to_ffi();
        let references_to = self.relation.to_ffi();

        Ingredient {
            name,
            alias,
            quantity,
            note,
            modifiers: self.modifiers().bits(),
            references_to,
        }
    }
}

impl ToFfi for cooklang::model::ComponentRelation {
    type Target = isize;

    fn to_ffi(&self) -> Self::Target {
        match &self {
            cooklang::model::ComponentRelation::Definition { .. } => -1,
            cooklang::model::ComponentRelation::Reference { references_to } => {
                *references_to as isize
            }
        }
    }
}

impl FfiFree for Ingredient {
    fn ffi_free(&mut self) {
        self.name.ffi_free();
        self.alias.ffi_free();
        self.quantity.ffi_free();
        self.note.ffi_free();
    }
}

#[repr(C)]
pub struct Cookware {
    name: CCString,
    /// nullable
    alias: CCString,
    /// nullable
    quantity: *const QuantityValue,
    /// nullable
    note: CCString,
    modifiers: u32,
    /// index to definition. -1 if this is the definition
    references_to: isize,
}

impl ToFfi for cooklang::model::Cookware {
    type Target = Cookware;

    fn to_ffi(&self) -> Self::Target {
        Cookware {
            name: self.name.as_str().to_ffi(),
            alias: self.alias.as_deref().to_ffi(),
            quantity: self
                .quantity
                .as_ref()
                .map_or(ptr::null(), |q| Box::into_raw(Box::new(q.to_ffi()))),
            note: self.note.as_deref().to_ffi(),
            modifiers: self.modifiers().bits(),
            references_to: self.relation.to_ffi(),
        }
    }
}

impl FfiFree for Cookware {
    fn ffi_free(&mut self) {
        self.name.ffi_free();
        self.alias.ffi_free();
        if !self.quantity.is_null() {
            unsafe { &mut *(self.quantity as *mut QuantityValue) }.ffi_free();
        }
        self.note.ffi_free();
    }
}

#[repr(C)]
pub struct Timer {
    /// nullable
    name: CCString,
    quantity: Quantity,
}

impl ToFfi for cooklang::model::Timer {
    type Target = Timer;

    fn to_ffi(&self) -> Self::Target {
        Timer {
            name: self.name.as_deref().to_ffi(),
            quantity: self.quantity.to_ffi(),
        }
    }
}

impl FfiFree for Timer {
    fn ffi_free(&mut self) {
        self.name.ffi_free();
        self.quantity.ffi_free();
    }
}

#[repr(C)]
pub struct Quantity {
    value: QuantityValue,
    /// nullable
    unit: *const c_char,
}

impl ToFfi for cooklang::quantity::Quantity {
    type Target = Quantity;

    fn to_ffi(&self) -> Self::Target {
        Quantity {
            value: self.value.to_ffi(),
            unit: opt_cstring_ptr(self.unit_text()),
        }
    }
}

impl ToFfi for Option<cooklang::quantity::Quantity> {
    type Target = *const Quantity;

    fn to_ffi(&self) -> Self::Target {
        self.as_ref()
            .map_or(ptr::null(), |q| Box::into_raw(Box::new(q.to_ffi())))
    }
}

impl FfiFree for Quantity {
    fn ffi_free(&mut self) {
        self.value.ffi_free();
        if !self.unit.is_null() {
            unsafe { drop(CString::from_raw(self.unit as *mut c_char)) }
        }
    }
}

impl FfiFree for *const Quantity {
    fn ffi_free(&mut self) {
        if !self.is_null() {
            unsafe { (*(*self as *mut Quantity)).ffi_free() };
        }
    }
}

/// cbindgen:prefix-with-name
#[repr(C)]
pub enum QuantityValue {
    /// Cannot be scaled
    Fixed(Value),
    /// Scaling is linear to the number of servings
    Linear(Value),
    /// Scaling is in defined steps of the number of servings
    ByServings {
        values: *const Value,
        values_len: usize,
    },
}

impl ToFfi for cooklang::quantity::QuantityValue {
    type Target = QuantityValue;

    fn to_ffi(&self) -> Self::Target {
        match self {
            cooklang::quantity::QuantityValue::Fixed(val) => QuantityValue::Fixed(val.to_ffi()),
            cooklang::quantity::QuantityValue::Linear(val) => QuantityValue::Linear(val.to_ffi()),
            cooklang::quantity::QuantityValue::ByServings(values) => {
                let (values, values_len) = ffi_vec(values.iter(), |v| v.to_ffi());
                QuantityValue::ByServings { values, values_len }
            }
        }
    }
}

impl FfiFree for QuantityValue {
    fn ffi_free(&mut self) {
        match self {
            QuantityValue::Fixed(v) => v.ffi_free(),
            QuantityValue::Linear(v) => v.ffi_free(),
            QuantityValue::ByServings { values, values_len } => {
                unsafe { drop(ffi_vec_from_raw(*values, *values_len)) };
            }
        }
    }
}

/// cbindgen:prefix-with-name
#[repr(C)]
pub enum Value {
    /// Numeric
    Number(f64),
    /// Range
    Range { from: f64, to: f64 },
    /// Text
    ///
    /// It is not possible to operate with this variant.
    Text(CCString),
}

impl ToFfi for cooklang::quantity::Value {
    type Target = Value;

    fn to_ffi(&self) -> Self::Target {
        match self {
            cooklang::quantity::Value::Number(n) => Value::Number(*n),
            cooklang::quantity::Value::Range(r) => Value::Range {
                from: *r.start(),
                to: *r.end(),
            },
            cooklang::quantity::Value::Text(s) => Value::Text(s.as_str().to_ffi()),
        }
    }
}

impl FfiFree for Value {
    fn ffi_free(&mut self) {
        match self {
            Value::Text(s) => s.ffi_free(),
            _ => {}
        }
    }
}
