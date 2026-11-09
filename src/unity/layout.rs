use std::{cell::RefCell, collections::HashMap};

use super::{
    scene::{Key, Model, Role, script_guid},
    ui::{
        self, Child, Fit, Group, Offer, REFERENCE_PIXELS_PER_UNIT, Rect, RectFields, Script, Sizes, plain_text, script_of,
        sizes_from, text_alignment, text_sizes,
    },
    value::Lookup,
    yaml::{Document, Node},
};

const DEEPEST_NESTING: usize = 3;
const DEEPEST_HIERARCHY: usize = 64;
const TRANSFORM: u32 = 4;
const RECT_TRANSFORM: u32 = 224;
const PREFAB_INSTANCE: u32 = 1001;
const TEXTURE: i64 = 2800000;
const MATERIAL_ONLY_ALPHA: f64 = 0.3;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Matrix([[f64; 4]; 3]);

impl Matrix {
    const IDENTITY: Matrix = Matrix([[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]]);

    fn trs(position: [f64; 3], rotation: [f64; 4], scale: [f64; 3]) -> Self {
        let [x, y, z, w] = rotation;
        let r = [
            [1. - 2. * (y * y + z * z), 2. * (x * y - z * w), 2. * (x * z + y * w)],
            [2. * (x * y + z * w), 1. - 2. * (x * x + z * z), 2. * (y * z - x * w)],
            [2. * (x * z - y * w), 2. * (y * z + x * w), 1. - 2. * (x * x + y * y)],
        ];
        let mut m = [[0.; 4]; 3];
        for row in 0..3 {
            for column in 0..3 {
                m[row][column] = r[row][column] * scale[column];
            }
            m[row][3] = position[row];
        }
        Matrix(m)
    }

    fn then(self, local: Matrix) -> Matrix {
        let (a, b) = (self.0, local.0);
        let mut m = [[0.; 4]; 3];
        for row in 0..3 {
            for column in 0..4 {
                m[row][column] = (0..3).map(|k| a[row][k] * b[k][column]).sum::<f64>() + if column == 3 { a[row][3] } else { 0. };
            }
        }
        Matrix(m)
    }

    fn point(&self, p: [f64; 3]) -> [f64; 3] {
        let m = self.0;
        [0, 1, 2].map(|row| m[row][0] * p[0] + m[row][1] * p[1] + m[row][2] * p[2] + m[row][3])
    }

    fn axis_length(&self, column: usize) -> f64 {
        (0..3).map(|row| self.0[row][column].powi(2)).sum::<f64>().sqrt()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum View {
    Front,
    Above,
}

impl View {
    fn project(self, p: [f64; 3]) -> (f64, f64) {
        match self {
            View::Front => (p[0], p[1]),
            View::Above => (p[0], p[2]),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlacedSprite {
    pub(crate) guid: String,
    pub(crate) file_id: i64,
    pub(crate) at: (f64, f64),
    pub(crate) scale: (f64, f64),
    pub(crate) flip: (bool, bool),
    pub(crate) order: i64,
    pub(crate) alpha: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Shape {
    Circle { center: (f64, f64), radius: f64 },
    Box { center: (f64, f64), size: (f64, f64) },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Marker {
    Plain,
    Camera,
    Light,
    Canvas,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Paint {
    Fill,
    Sprite { guid: String, file_id: i64, sliced: bool, keep_aspect: bool, unit: f64 },
    Texture { guid: String },
    Text { text: String, size: f64, align: [f64; 2] },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Graphic {
    pub(crate) area: [f64; 4],
    pub(crate) color: [f64; 4],
    pub(crate) paint: Paint,
}

impl Paint {
    pub(crate) fn sprite(&self) -> Option<(String, i64)> {
        match self {
            Paint::Sprite { guid, file_id, .. } => Some((guid.clone(), *file_id)),
            Paint::Texture { guid } => Some((guid.clone(), TEXTURE)),
            Paint::Fill | Paint::Text { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Spot {
    pub(crate) key: Key,
    pub(crate) name: String,
    pub(crate) at: (f64, f64),
    pub(crate) sprites: Vec<PlacedSprite>,
    pub(crate) shapes: Vec<(String, Shape)>,
    pub(crate) camera: Option<f64>,
    pub(crate) marker: Marker,
    pub(crate) rect: Option<[f64; 4]>,
    pub(crate) graphics: Vec<Graphic>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Layout {
    pub(crate) spots: Vec<Spot>,
}

impl Layout {
    pub(crate) fn spot(&self, key: Key) -> Option<&Spot> {
        self.spots.iter().find(|spot| spot.key == key)
    }
}

pub(crate) fn view_for(documents: &[Document]) -> View {
    let flat = documents.iter().any(|document| {
        matches!(
            document.type_name.as_str(),
            "SpriteRenderer" | "CircleCollider2D" | "BoxCollider2D" | "Rigidbody2D" | "Tilemap" | "CapsuleCollider2D" | "Canvas"
        )
    });
    let solid = documents.iter().any(|document| matches!(document.type_name.as_str(), "MeshRenderer" | "MeshFilter" | "Terrain"));
    if solid && !flat { View::Above } else { View::Front }
}

type Overrides = HashMap<(i64, String), String>;

struct File<'a> {
    by_id: HashMap<i64, &'a Document>,
    documents: &'a [Document],
    overrides: Overrides,
    arranged: RefCell<HashMap<i64, Vec<(i64, RectFields)>>>,
}

impl<'a> File<'a> {
    fn new(documents: &'a [Document], overrides: Overrides) -> Self {
        Self {
            by_id: documents.iter().map(|document| (document.file_id, document)).collect(),
            documents,
            overrides,
            arranged: RefCell::new(HashMap::new()),
        }
    }

    fn document(&self, id: i64) -> Option<&'a Document> {
        self.by_id.get(&id).copied()
    }

    fn text(&self, id: i64, path: &str) -> Option<String> {
        if let Some(value) = self.overrides.get(&(id, path.to_owned())) {
            return Some(value.clone());
        }
        let keys: Vec<&str> = path.split('.').collect();
        self.document(id)?.body.at(&keys)?.text().map(ToOwned::to_owned)
    }

    fn read(&self, id: i64, path: &str) -> Option<f64> {
        self.text(id, path)?.trim().parse().ok()
    }

    fn reference(&self, id: i64, key: &str) -> Option<(String, i64)> {
        let guid = self.text(id, &format!("{key}.guid")).filter(|guid| !guid.is_empty())?;
        let file_id = self.text(id, &format!("{key}.fileID"))?.trim().parse().ok().filter(|file_id: &i64| *file_id != 0)?;
        Some((guid, file_id))
    }

    fn components(&self, game_object: i64) -> Vec<&'a Document> {
        self.document(game_object)
            .map(|owner| {
                owner
                    .body
                    .get("m_Component")
                    .map(Node::items)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|entry| file_id(entry.get("component").or_else(|| entry.entries().first().map(|(_, value)| value))))
                    .filter_map(|id| self.document(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn script(&self, game_object: i64, wanted: &[Script]) -> Option<(&'a Document, Script)> {
        self.components(game_object)
            .into_iter()
            .filter(|component| self.read(component.file_id, "m_Enabled") != Some(0.))
            .find_map(|component| script_of(component).filter(|script| wanted.contains(script)).map(|script| (component, script)))
    }

    fn game_object(&self, transform: i64) -> Option<i64> {
        file_id(self.document(transform)?.body.get("m_GameObject"))
    }

    fn children(&self, transform: i64) -> Vec<i64> {
        self.document(transform)
            .and_then(|document| document.body.get("m_Children"))
            .map(Node::items)
            .unwrap_or_default()
            .iter()
            .filter_map(|child| file_id(Some(child)))
            .collect()
    }
}

fn file_id(node: Option<&Node>) -> Option<i64> {
    node?.get("fileID")?.integer().filter(|&id| id != 0)
}

fn overrides_of(instance: &Node) -> Overrides {
    instance
        .at(&["m_Modification", "m_Modifications"])
        .map(Node::items)
        .unwrap_or_default()
        .iter()
        .filter_map(|change| {
            let target = file_id(change.get("target"))?;
            let path = change.get("propertyPath")?.text()?.to_owned();
            let value = change.get("value")?.text()?.to_owned();
            Some(((target, path), value))
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    matrix: Matrix,
    rect: Option<Rect>,
    active: bool,
    alpha: f64,
    driven: Option<RectFields>,
}

impl Frame {
    const ROOT: Frame = Frame {
        matrix: Matrix::IDENTITY,
        rect: None,
        active: true,
        alpha: 1.,
        driven: None,
    };

    fn area(&self, view: View) -> Option<[f64; 4]> {
        let rect = self.rect?;
        let corners = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]].map(|[x, y]| {
            view.project(self.matrix.point([rect.min[0] + x * rect.size[0], rect.min[1] + y * rect.size[1], 0.]))
        });
        let xs = corners.map(|corner| corner.0);
        let ys = corners.map(|corner| corner.1);
        let low = |values: [f64; 4]| values.into_iter().fold(f64::INFINITY, f64::min);
        let high = |values: [f64; 4]| values.into_iter().fold(f64::NEG_INFINITY, f64::max);
        Some([low(xs), low(ys), high(xs), high(ys)])
    }
}

struct Placer<'a> {
    library: &'a HashMap<String, Vec<Document>>,
    view: View,
}

impl<'a> Placer<'a> {
    fn local(&self, file: &File, transform: i64) -> Matrix {
        let triple = |key: &str, default: f64| ["x", "y", "z"].map(|axis| file.read(transform, &format!("{key}.{axis}")).unwrap_or(default));
        Matrix::trs(triple("m_LocalPosition", 0.), self.rotation(file, transform), triple("m_LocalScale", 1.))
    }

    fn rotation(&self, file: &File, transform: i64) -> [f64; 4] {
        let mut rotation = [0., 0., 0., 1.];
        for (ix, axis) in ["x", "y", "z", "w"].iter().enumerate() {
            if let Some(value) = file.read(transform, &format!("m_LocalRotation.{axis}")) {
                rotation[ix] = value;
            }
        }
        rotation
    }

    fn screen(&self, file: &File, game_object: i64) -> Option<[f64; 2]> {
        let canvas = file.components(game_object).into_iter().find(|component| component.type_name == "Canvas")?;
        if file.read(canvas.file_id, "m_RenderMode") == Some(2.) {
            return None;
        }
        let scaled = file.script(game_object, &[Script::Scaler]).filter(|(scaler, _)| file.read(scaler.file_id, "m_UiScaleMode") == Some(1.));
        Some(match scaled {
            Some((scaler, _)) => [
                file.read(scaler.file_id, "m_ReferenceResolution.x").filter(|width| *width > 0.).unwrap_or(ui::DEFAULT_SCREEN[0]),
                file.read(scaler.file_id, "m_ReferenceResolution.y").filter(|height| *height > 0.).unwrap_or(ui::DEFAULT_SCREEN[1]),
            ],
            None => ui::DEFAULT_SCREEN,
        })
    }

    fn frame(&self, file: &File<'a>, transform: i64, root: Frame, depth: usize) -> Frame {
        let Some(document) = file.document(transform) else {
            return root;
        };
        let parent = file_id(document.body.get("m_Father"));
        let above = match parent {
            Some(parent) if depth < DEEPEST_HIERARCHY => self.frame(file, parent, root, depth + 1),
            _ => root,
        };
        let game_object = file_id(document.body.get("m_GameObject"));
        let active = above.active && game_object.is_none_or(|owner| file.read(owner, "m_IsActive") != Some(0.));
        let fade = game_object
            .and_then(|owner| file.components(owner).into_iter().find(|component| component.type_name == "CanvasGroup"))
            .and_then(|group| file.read(group.file_id, "m_Alpha"))
            .unwrap_or(1.);
        let alpha = above.alpha * fade;
        if document.class_id != RECT_TRANSFORM {
            return Frame {
                matrix: above.matrix.then(self.local(file, transform)),
                rect: None,
                active,
                alpha,
                driven: None,
            };
        }
        let own = RectFields::read(&|path| file.read(transform, path));
        let depth_z = file.read(transform, "m_LocalPosition.z").unwrap_or(0.);
        let scale = ["x", "y", "z"].map(|axis| file.read(transform, &format!("m_LocalScale.{axis}")).unwrap_or(1.));
        let screen = game_object.filter(|_| above.rect.is_none()).and_then(|owner| self.screen(file, owner));
        let (at, rect, rotation, scale) = match (screen, above.rect) {
            (Some(size), _) => ([own.pivot[0] * size[0], own.pivot[1] * size[1]], own.own(size), [0., 0., 0., 1.], [1.; 3]),
            (None, Some(parent_rect)) => {
                let driven = if parent.is_none() { root.driven } else { parent.and_then(|parent| self.driven(file, parent, transform, parent_rect, depth)) };
                let fields = self.fitted(file, transform, game_object, driven.unwrap_or(own), driven.is_some(), parent_rect, depth);
                let (at, rect) = fields.place(parent_rect);
                (at, rect, self.rotation(file, transform), scale)
            }
            (None, None) => {
                let at = [file.read(transform, "m_LocalPosition.x").unwrap_or(0.), file.read(transform, "m_LocalPosition.y").unwrap_or(0.)];
                (at, own.own(own.size_delta), self.rotation(file, transform), scale)
            }
        };
        Frame {
            matrix: above.matrix.then(Matrix::trs([at[0], at[1], depth_z], rotation, scale)),
            rect: Some(rect),
            active,
            alpha,
            driven: None,
        }
    }

    fn fitted(&self, file: &File<'a>, transform: i64, game_object: Option<i64>, fields: RectFields, driven: bool, parent: Rect, depth: usize) -> RectFields {
        let Some((fitter, _)) = game_object.and_then(|owner| file.script(owner, &[Script::Fitter])) else {
            return fields;
        };
        let fit = Fit::read(&|path| file.read(fitter.file_id, path));
        let controlled = |axis: usize| {
            driven
                && file
                    .document(transform)
                    .and_then(|document| file_id(document.body.get("m_Father")))
                    .and_then(|parent| file.game_object(parent))
                    .and_then(|owner| self.group(file, owner))
                    .is_some_and(|group| group.controls(axis))
        };
        let sizes = self.sizes(file, transform, depth);
        let mut fitted = fields;
        for axis in 0..2 {
            if let Some(size) = fit.size(axis, sizes).filter(|_| !controlled(axis)) {
                fitted = fitted.sized(axis, size, parent);
            }
        }
        fitted
    }

    fn group(&self, file: &File, game_object: i64) -> Option<Group> {
        let (document, script) = file.script(game_object, &[Script::Horizontal, Script::Vertical, Script::Grid])?;
        Group::read(script, &|path| file.read(document.file_id, path))
    }

    fn resolved(&self, file: &File<'a>, transform: i64) -> Option<(File<'a>, i64)> {
        let document = file.document(transform)?;
        if !document.stripped {
            return None;
        }
        let instance = file.document(file_id(document.body.get("m_PrefabInstance"))?)?;
        let guid = instance.body.at(&["m_SourcePrefab", "guid"]).and_then(Node::text)?;
        let documents = self.library.get(guid)?;
        let source = file_id(document.body.get("m_CorrespondingSourceObject"))?;
        Some((File::new(documents, overrides_of(&instance.body)), source))
    }

    fn sizes(&self, file: &File<'a>, transform: i64, depth: usize) -> [Sizes; 2] {
        if depth > DEEPEST_HIERARCHY {
            return [Sizes::default(); 2];
        }
        if let Some((inner, source)) = self.resolved(file, transform) {
            return self.sizes(&inner, source, depth + 1);
        }
        let Some(owner) = file.game_object(transform) else {
            return [Sizes::default(); 2];
        };
        let mut offers = Vec::new();
        for component in file.components(owner) {
            if file.read(component.file_id, "m_Enabled") == Some(0.) {
                continue;
            }
            let read = |path: &str| file.read(component.file_id, path);
            match script_of(component) {
                Some(Script::LayoutElement) => offers.extend(Offer::element(&read)),
                Some(script @ (Script::Text | Script::TextMeshPro)) => {
                    let (text, size) = self.words(file, component.file_id, script);
                    offers.push(Offer::of(text_sizes(&text, size)));
                }
                Some(script @ (Script::Horizontal | Script::Vertical | Script::Grid)) => {
                    if let Some(group) = Group::read(script, &read) {
                        let children = self.laid_children(file, transform, depth);
                        let children: Vec<Child> = children.into_iter().map(|(_, child)| child).collect();
                        offers.push(Offer::of(group.sizes(&children)));
                    }
                }
                _ => {}
            }
        }
        sizes_from(&offers)
    }

    fn laid_children(&self, file: &File<'a>, transform: i64, depth: usize) -> Vec<(i64, Child)> {
        file.children(transform)
            .into_iter()
            .filter_map(|child| {
                let (fields, ignored, active) = match self.resolved(file, child) {
                    Some((inner, source)) => self.layout_facts(&inner, source),
                    None => self.layout_facts(file, child),
                };
                (!ignored && active).then(|| {
                    (
                        child,
                        Child {
                            fields,
                            sizes: self.sizes(file, child, depth + 1),
                        },
                    )
                })
            })
            .collect()
    }

    fn layout_facts(&self, file: &File, transform: i64) -> (RectFields, bool, bool) {
        let fields = RectFields::read(&|path| file.read(transform, path));
        let owner = file.game_object(transform);
        let ignored = owner.is_some_and(|owner| {
            file.script(owner, &[Script::LayoutElement])
                .is_some_and(|(element, _)| file.read(element.file_id, "m_IgnoreLayout") == Some(1.))
        });
        let active = owner.is_none_or(|owner| file.read(owner, "m_IsActive") != Some(0.));
        (fields, ignored, active)
    }

    fn driven(&self, file: &File<'a>, parent: i64, child: i64, parent_rect: Rect, depth: usize) -> Option<RectFields> {
        if let Some(found) = file.arranged.borrow().get(&parent) {
            return found.iter().find(|(id, _)| *id == child).map(|(_, fields)| *fields);
        }
        let group = file.game_object(parent).and_then(|owner| self.group(file, owner));
        let arranged = match group {
            Some(group) => {
                let children = self.laid_children(file, parent, depth);
                let placed: Vec<Child> = children.iter().map(|(_, child)| *child).collect();
                children.iter().map(|(id, _)| *id).zip(group.arrange(parent_rect, &placed)).collect()
            }
            None => Vec::new(),
        };
        let found = arranged.iter().find(|(id, _)| *id == child).map(|(_, fields)| *fields);
        file.arranged.borrow_mut().insert(parent, arranged);
        found
    }

    fn words(&self, file: &File, component: i64, script: Script) -> (String, f64) {
        match script {
            Script::Text => (
                plain_text(&file.text(component, "m_Text").unwrap_or_default()),
                file.read(component, "m_FontData.m_FontSize").unwrap_or(14.),
            ),
            _ => (
                plain_text(&file.text(component, "m_text").unwrap_or_default()),
                file.read(component, "m_fontSize").unwrap_or(36.),
            ),
        }
    }

    fn color(&self, file: &File, component: i64, key: &str) -> Option<[f64; 4]> {
        let channels = ["r", "g", "b", "a"].map(|channel| file.read(component, &format!("{key}.{channel}")));
        channels.iter().any(Option::is_some).then(|| channels.map(|channel| channel.unwrap_or(1.)))
    }

    fn graphic(&self, file: &File<'a>, component: &Document, script: Script, root: Frame) -> Option<Graphic> {
        if file.read(component.file_id, "m_Enabled") == Some(0.) {
            return None;
        }
        let owner = file_id(component.body.get("m_GameObject"))?;
        let transform = self.transform_of(file, owner)?;
        let frame = self.frame(file, transform, root, 0);
        if !frame.active {
            return None;
        }
        frame.rect?;
        let mut area = frame.area(self.view)?;
        let id = component.file_id;
        let mut color = self.color(file, id, "m_Color").unwrap_or([1.; 4]);
        let paint = match script {
            Script::Image => {
                match file.reference(id, "m_Sprite") {
                    Some((guid, file_id)) => Paint::Sprite {
                        guid,
                        file_id,
                        sliced: matches!(file.read(id, "m_Type"), Some(1. | 2.)),
                        keep_aspect: file.read(id, "m_PreserveAspect") == Some(1.),
                        unit: frame.matrix.axis_length(0) * REFERENCE_PIXELS_PER_UNIT / file.read(id, "m_PixelsPerUnitMultiplier").filter(|value| *value > 0.).unwrap_or(1.),
                    },
                    None => {
                        if file.text(id, "m_Material.guid").is_some_and(|guid| !guid.is_empty()) {
                            color[3] *= MATERIAL_ONLY_ALPHA;
                        }
                        Paint::Fill
                    }
                }
            }
            Script::RawImage => match file.text(id, "m_Texture.guid") {
                Some(guid) if !guid.is_empty() => Paint::Texture { guid },
                _ => Paint::Fill,
            },
            Script::Text | Script::TextMeshPro => {
                let (text, size) = self.words(file, id, script);
                if script == Script::TextMeshPro {
                    color = self.color(file, id, "m_fontColor").unwrap_or(color);
                    let margin = ["x", "y", "z", "w"].map(|side| file.read(id, &format!("m_margin.{side}")).unwrap_or(0.));
                    let scale = frame.matrix.axis_length(0);
                    area = [area[0] + margin[0] * scale, area[1] + margin[3] * scale, area[2] - margin[2] * scale, area[3] - margin[1] * scale];
                }
                Paint::Text {
                    text,
                    size: size * frame.matrix.axis_length(1),
                    align: text_alignment(script, &|path| file.read(id, path)),
                }
            }
            _ => return None,
        };
        color[3] *= frame.alpha;
        Some(Graphic { area, color, paint })
    }

    fn renderers(&self, file: &File<'a>, documents: &[&'a Document], root: Frame, nesting: usize, out: &mut Placed) {
        for document in documents {
            if document.stripped {
                continue;
            }
            match document.type_name.as_str() {
                "SpriteRenderer" => {
                    let Some(owner) = file_id(document.body.get("m_GameObject")) else {
                        continue;
                    };
                    let Some(transform) = self.transform_of(file, owner) else {
                        continue;
                    };
                    let id = document.file_id;
                    let Some((guid, sprite_id)) = file.reference(id, "m_Sprite") else {
                        continue;
                    };
                    let frame = self.frame(file, transform, root, 0);
                    if !frame.active {
                        continue;
                    }
                    let world = frame.matrix;
                    out.sprites.push(PlacedSprite {
                        guid,
                        file_id: sprite_id,
                        at: self.view.project(world.point([0., 0., 0.])),
                        scale: (world.axis_length(0), world.axis_length(1)),
                        flip: (file.read(id, "m_FlipX") == Some(1.), file.read(id, "m_FlipY") == Some(1.)),
                        order: file.read(id, "m_SortingOrder").unwrap_or(0.) as i64,
                        alpha: file.read(id, "m_Color.a").unwrap_or(1.),
                    });
                }
                "CircleCollider2D" | "BoxCollider2D" | "CapsuleCollider2D" => {
                    if let Some(shape) = self.shape(file, document, root) {
                        out.shapes.push((super::names::component_title(&document.type_name), shape));
                    }
                }
                "MonoBehaviour" => {
                    if let Some(script @ (Script::Image | Script::RawImage | Script::Text | Script::TextMeshPro)) = script_of(document)
                        && let Some(graphic) = self.graphic(file, document, script, root)
                    {
                        out.graphics.push(graphic);
                    }
                }
                _ if document.class_id == PREFAB_INSTANCE && nesting < DEEPEST_NESTING => {
                    let above = self.instance_frame(file, document, root);
                    self.instance(&document.body, above, nesting + 1, out);
                }
                _ => {}
            }
        }
    }

    fn instance_frame(&self, file: &File<'a>, instance: &Document, root: Frame) -> Frame {
        let Some(parent) = file_id(instance.body.at(&["m_Modification", "m_TransformParent"])) else {
            return root;
        };
        let above = self.frame(file, parent, root, 0);
        let stripped = file.documents.iter().find(|document| {
            document.stripped && document.class_id == RECT_TRANSFORM && file_id(document.body.get("m_PrefabInstance")) == Some(instance.file_id)
        });
        let driven = match (stripped, above.rect) {
            (Some(stripped), Some(rect)) => self.driven(file, parent, stripped.file_id, rect, 0),
            _ => None,
        };
        Frame { driven, ..above }
    }

    fn transform_of(&self, file: &File, game_object: i64) -> Option<i64> {
        file.components(game_object)
            .into_iter()
            .find(|component| matches!(component.class_id, TRANSFORM | RECT_TRANSFORM))
            .map(|component| component.file_id)
    }

    fn shape(&self, file: &File<'a>, collider: &Document, root: Frame) -> Option<Shape> {
        let owner = file_id(collider.body.get("m_GameObject"))?;
        let transform = self.transform_of(file, owner)?;
        let world = self.frame(file, transform, root, 0).matrix;
        let id = collider.file_id;
        let offset = [file.read(id, "m_Offset.x").unwrap_or(0.), file.read(id, "m_Offset.y").unwrap_or(0.)];
        let center = self.view.project(world.point([offset[0], offset[1], 0.]));
        let (sx, sy) = (world.axis_length(0), world.axis_length(1));
        Some(match collider.type_name.as_str() {
            "CircleCollider2D" => Shape::Circle {
                center,
                radius: file.read(id, "m_Radius").unwrap_or(0.5) * sx.max(sy),
            },
            _ => Shape::Box {
                center,
                size: (file.read(id, "m_Size.x").unwrap_or(1.) * sx, file.read(id, "m_Size.y").unwrap_or(1.) * sy),
            },
        })
    }

    fn instance(&self, body: &Node, above: Frame, nesting: usize, out: &mut Placed) {
        let Some(documents) = body.at(&["m_SourcePrefab", "guid"]).and_then(Node::text).and_then(|guid| self.library.get(guid)) else {
            return;
        };
        let file = File::new(documents, overrides_of(body));
        let all: Vec<&Document> = documents.iter().collect();
        self.renderers(&file, &all, above, nesting, out);
    }

    fn instance_origin(&self, body: &Node, above: Frame) -> ((f64, f64), Option<[f64; 4]>, bool) {
        let Some(documents) = body.at(&["m_SourcePrefab", "guid"]).and_then(Node::text).and_then(|guid| self.library.get(guid)) else {
            let overrides = overrides_of(body);
            let read = |axis: &str| {
                overrides
                    .iter()
                    .find(|((_, path), _)| path == &format!("m_LocalPosition.{axis}"))
                    .and_then(|(_, value)| value.trim().parse().ok())
                    .unwrap_or(0.)
            };
            return (self.view.project(above.matrix.point([read("x"), read("y"), read("z")])), None, false);
        };
        let file = File::new(documents, overrides_of(body));
        let root = documents
            .iter()
            .find(|document| matches!(document.class_id, TRANSFORM | RECT_TRANSFORM) && file_id(document.body.get("m_Father")).is_none());
        match root {
            Some(root) => {
                let frame = self.frame(&file, root.file_id, above, 0);
                let canvas = file.game_object(root.file_id).is_some_and(|owner| self.screen(&file, owner).is_some()) && above.rect.is_none();
                (self.view.project(frame.matrix.point([0., 0., 0.])), frame.area(self.view), canvas)
            }
            None => (self.view.project(above.matrix.point([0., 0., 0.])), None, false),
        }
    }
}

#[derive(Default)]
struct Placed {
    sprites: Vec<PlacedSprite>,
    shapes: Vec<(String, Shape)>,
    graphics: Vec<Graphic>,
}

pub(crate) fn layout(documents: &[Document], model: &Model, library: &HashMap<String, Vec<Document>>, lookup: &Lookup, view: View) -> Layout {
    let placer = Placer { library, view };
    let file = File::new(documents, Overrides::new());
    let mut spots = Vec::new();
    for item in &model.items {
        let mut placed = Placed::default();
        let mut camera = None;
        let mut marker = Marker::Plain;
        let (at, rect) = match &item.role {
            Role::Instance { .. } => {
                let Some(document) = file.document(item.key) else {
                    continue;
                };
                let above = placer.instance_frame(&file, document, Frame::ROOT);
                placer.instance(&document.body, above, 1, &mut placed);
                let (at, rect, canvas) = placer.instance_origin(&document.body, above);
                if canvas {
                    marker = Marker::Canvas;
                }
                (at, rect)
            }
            _ => {
                let Some(transform) = placer.transform_of(&file, item.key) else {
                    continue;
                };
                let own = file.components(item.key);
                placer.renderers(&file, &own, Frame::ROOT, DEEPEST_NESTING, &mut placed);
                for component in &own {
                    match component.type_name.as_str() {
                        "Camera" => {
                            marker = Marker::Camera;
                            if component.body.get("orthographic").and_then(Node::text) == Some("1") {
                                camera = component.body.get("orthographic size").and_then(Node::number);
                            }
                        }
                        "Light" => marker = Marker::Light,
                        "MonoBehaviour" => {
                            let light = script_guid(&component.body)
                                .and_then(|guid| lookup.script_name(guid))
                                .is_some_and(|name| name.replace(' ', "") == "Light2D");
                            if light {
                                marker = Marker::Light;
                            }
                        }
                        _ => {}
                    }
                }
                let frame = placer.frame(&file, transform, Frame::ROOT, 0);
                let is_root = file.document(transform).is_some_and(|document| file_id(document.body.get("m_Father")).is_none());
                if is_root && placer.screen(&file, item.key).is_some() {
                    marker = Marker::Canvas;
                }
                (view.project(frame.matrix.point([0., 0., 0.])), frame.area(view))
            }
        };
        spots.push(Spot {
            key: item.key,
            name: item.name.clone(),
            at,
            sprites: placed.sprites,
            shapes: placed.shapes,
            camera,
            marker,
            rect,
            graphics: placed.graphics,
        });
    }
    Layout { spots }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Marker, Paint, Shape, View, layout, view_for};
    use crate::unity::{
        names::UnityKind,
        scene::{Model, object_names},
        value::Lookup,
        yaml::documents,
    };

    const SCENE: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &10
GameObject:
  m_Component:
  - component: {fileID: 11}
  - component: {fileID: 12}
  - component: {fileID: 13}
  m_Name: Slime
--- !u!4 &11
Transform:
  m_GameObject: {fileID: 10}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 2, y: 1, z: 0}
  m_LocalScale: {x: 2, y: 2, z: 1}
  m_Children:
  - {fileID: 21}
  m_Father: {fileID: 0}
--- !u!212 &12
SpriteRenderer:
  m_GameObject: {fileID: 10}
  m_Sprite: {fileID: 21300000, guid: aaaabbbbccccddddeeeeffff00001111, type: 3}
  m_SortingOrder: 2
--- !u!58 &13
CircleCollider2D:
  m_GameObject: {fileID: 10}
  m_Radius: 0.5
  m_Offset: {x: 0, y: 0.5}
--- !u!1 &20
GameObject:
  m_Component:
  - component: {fileID: 21}
  - component: {fileID: 22}
  m_Name: Main Camera
--- !u!4 &21
Transform:
  m_GameObject: {fileID: 20}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 1, y: 0, z: -10}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Father: {fileID: 11}
--- !u!20 &22
Camera:
  m_GameObject: {fileID: 20}
  orthographic: 1
  orthographic size: 5
";

    #[test]
    fn children_sit_where_their_parents_put_them() {
        let docs = documents(SCENE);
        let mut lookup = Lookup::default();
        lookup.objects = object_names(&docs, &lookup.assets);
        let model = Model::from_documents(UnityKind::Scene, &docs, "Assets/Level1.unity", &lookup);
        assert_eq!(view_for(&docs), View::Front);
        let placed = layout(&docs, &model, &HashMap::new(), &lookup, View::Front);
        let slime = placed.spot(10).expect("slime");
        assert_eq!(slime.at, (2., 1.));
        assert_eq!(slime.sprites[0].scale, (2., 2.));
        assert_eq!(slime.sprites[0].order, 2);
        assert_eq!(slime.shapes[0].1, Shape::Circle { center: (2., 2.), radius: 1. });
        let camera = placed.spot(20).expect("camera");
        assert_eq!(camera.at, (4., 1.));
        assert_eq!(camera.camera, Some(5.));
        assert_eq!(camera.marker, Marker::Camera);
    }

    const MENU: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1
GameObject:
  m_Component:
  - component: {fileID: 2}
  - component: {fileID: 3}
  - component: {fileID: 4}
  m_Name: Menu
--- !u!224 &2
RectTransform:
  m_GameObject: {fileID: 1}
  m_LocalScale: {x: 0, y: 0, z: 0}
  m_Children:
  - {fileID: 12}
  - {fileID: 22}
  m_Father: {fileID: 0}
  m_AnchorMin: {x: 0, y: 0}
  m_AnchorMax: {x: 0, y: 0}
  m_SizeDelta: {x: 0, y: 0}
  m_Pivot: {x: 0, y: 0}
--- !u!223 &3
Canvas:
  m_GameObject: {fileID: 1}
  m_RenderMode: 0
--- !u!114 &4
MonoBehaviour:
  m_GameObject: {fileID: 1}
  m_Script: {fileID: 11500000, guid: 0cd44c1031e13a943bb63640046fad76, type: 3}
  m_UiScaleMode: 1
  m_ReferenceResolution: {x: 1600, y: 900}
--- !u!1 &11
GameObject:
  m_Component:
  - component: {fileID: 12}
  - component: {fileID: 13}
  m_Name: Background
--- !u!224 &12
RectTransform:
  m_GameObject: {fileID: 11}
  m_Father: {fileID: 2}
  m_AnchorMin: {x: 0, y: 0}
  m_AnchorMax: {x: 1, y: 1}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: -100, y: -100}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!114 &13
MonoBehaviour:
  m_GameObject: {fileID: 11}
  m_Script: {fileID: 11500000, guid: fe87c0e1cc204ed48ad3b37840f39efc, type: 3}
  m_Color: {r: 0.5, g: 0, b: 1, a: 1}
  m_Sprite: {fileID: 0}
--- !u!1 &21
GameObject:
  m_Component:
  - component: {fileID: 22}
  - component: {fileID: 23}
  m_Name: Buttons
--- !u!224 &22
RectTransform:
  m_GameObject: {fileID: 21}
  m_Children:
  - {fileID: 32}
  - {fileID: 42}
  m_Father: {fileID: 2}
  m_AnchorMin: {x: 0.5, y: 0.5}
  m_AnchorMax: {x: 0.5, y: 0.5}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 400, y: 300}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!114 &23
MonoBehaviour:
  m_GameObject: {fileID: 21}
  m_EditorClassIdentifier: UnityEngine.UI::UnityEngine.UI.VerticalLayoutGroup
  m_Script: {fileID: 11500000, guid: 59f8146938fff824cb5fd77236b75775, type: 3}
  m_ChildAlignment: 1
  m_Spacing: 20
  m_ChildForceExpandWidth: 0
  m_ChildForceExpandHeight: 0
  m_ChildControlWidth: 0
  m_ChildControlHeight: 0
--- !u!1 &31
GameObject:
  m_Component:
  - component: {fileID: 32}
  - component: {fileID: 33}
  m_Name: Play
--- !u!224 &32
RectTransform:
  m_GameObject: {fileID: 31}
  m_Father: {fileID: 22}
  m_AnchorMin: {x: 0, y: 1}
  m_AnchorMax: {x: 0, y: 1}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 300, y: 80}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!114 &33
MonoBehaviour:
  m_GameObject: {fileID: 31}
  m_EditorClassIdentifier: Unity.TextMeshPro::TMPro.TextMeshProUGUI
  m_text: \"<b>Play</b>\"
  m_fontSize: 40
  m_fontColor: {r: 1, g: 1, b: 1, a: 1}
  m_HorizontalAlignment: 2
  m_VerticalAlignment: 512
--- !u!1 &41
GameObject:
  m_Component:
  - component: {fileID: 42}
  m_Name: Quit
  m_IsActive: 1
--- !u!224 &42
RectTransform:
  m_GameObject: {fileID: 41}
  m_Father: {fileID: 22}
  m_AnchorMin: {x: 0, y: 1}
  m_AnchorMax: {x: 0, y: 1}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 300, y: 80}
  m_Pivot: {x: 0.5, y: 0.5}
";

    #[test]
    fn a_screen_canvas_is_drawn_at_its_reference_size_with_its_layout() {
        let docs = documents(MENU);
        let mut lookup = Lookup::default();
        lookup.objects = object_names(&docs, &lookup.assets);
        let model = Model::from_documents(UnityKind::Prefab, &docs, "Assets/UI/Menu.prefab", &lookup);
        let placed = layout(&docs, &model, &HashMap::new(), &lookup, View::Front);
        let menu = placed.spot(1).expect("menu");
        assert_eq!(menu.marker, Marker::Canvas);
        assert_eq!(menu.rect, Some([0., 0., 1600., 900.]));
        let background = placed.spot(11).expect("background");
        assert_eq!(background.rect, Some([50., 50., 1550., 850.]));
        assert_eq!(background.graphics[0].paint, Paint::Fill);
        assert_eq!(background.graphics[0].color, [0.5, 0., 1., 1.]);
        let play = placed.spot(31).expect("play");
        assert_eq!(play.rect, Some([650., 520., 950., 600.]));
        let quit = placed.spot(41).expect("quit");
        assert_eq!(quit.rect, Some([650., 420., 950., 500.]));
        match &play.graphics[0].paint {
            Paint::Text { text, size, align } => {
                assert_eq!(text, "Play");
                assert_eq!(*size, 40.);
                assert_eq!(*align, [0.5, 0.5]);
            }
            other => panic!("not a text: {other:?}"),
        }
    }
}
