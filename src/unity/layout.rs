use std::collections::HashMap;

use super::{
    scene::{Key, Model, Role, script_guid},
    value::Lookup,
    yaml::{Document, Node},
};

const DEEPEST_NESTING: usize = 3;
const TRANSFORM: u32 = 4;
const RECT_TRANSFORM: u32 = 224;
const PREFAB_INSTANCE: u32 = 1001;

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
            "SpriteRenderer" | "CircleCollider2D" | "BoxCollider2D" | "Rigidbody2D" | "Tilemap" | "CapsuleCollider2D"
        )
    });
    let solid = documents.iter().any(|document| matches!(document.type_name.as_str(), "MeshRenderer" | "MeshFilter" | "Terrain"));
    if solid && !flat { View::Above } else { View::Front }
}

struct Placer<'a> {
    library: &'a HashMap<String, Vec<Document>>,
    lookup: &'a Lookup,
    view: View,
}

struct File<'a> {
    by_id: HashMap<i64, &'a Document>,
}

impl<'a> File<'a> {
    fn new(documents: &'a [Document]) -> Self {
        Self {
            by_id: documents.iter().map(|document| (document.file_id, document)).collect(),
        }
    }

    fn document(&self, id: i64) -> Option<&'a Document> {
        self.by_id.get(&id).copied()
    }
}

fn file_id(node: Option<&Node>) -> Option<i64> {
    node?.get("fileID")?.number().map(|id| id as i64).filter(|&id| id != 0)
}

fn vector<const N: usize>(node: Option<&Node>, axes: [&str; N], default: [f64; N]) -> [f64; N] {
    let mut out = default;
    for (ix, axis) in axes.iter().enumerate() {
        if let Some(value) = node.and_then(|node| node.get(axis)).and_then(Node::number) {
            out[ix] = value;
        }
    }
    out
}

type Overrides = HashMap<(i64, String), f64>;

fn local(transform: &Node, id: i64, overrides: &Overrides) -> Matrix {
    let mut position = vector(transform.get("m_LocalPosition"), ["x", "y", "z"], [0.; 3]);
    let mut rotation = vector(transform.get("m_LocalRotation"), ["x", "y", "z", "w"], [0., 0., 0., 1.]);
    let mut scale = vector(transform.get("m_LocalScale"), ["x", "y", "z"], [1.; 3]);
    for (ix, axis) in ["x", "y", "z"].iter().enumerate() {
        if let Some(value) = overrides.get(&(id, format!("m_LocalPosition.{axis}"))) {
            position[ix] = *value;
        }
        if let Some(value) = overrides.get(&(id, format!("m_LocalScale.{axis}"))) {
            scale[ix] = *value;
        }
    }
    for (ix, axis) in ["x", "y", "z", "w"].iter().enumerate() {
        if let Some(value) = overrides.get(&(id, format!("m_LocalRotation.{axis}"))) {
            rotation[ix] = *value;
        }
    }
    Matrix::trs(position, rotation, scale)
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
            let value = change.get("value")?.number()?;
            Some(((target, path), value))
        })
        .collect()
}

impl Placer<'_> {
    fn world(&self, file: &File, transform: i64, overrides: &Overrides, root: Matrix, depth: usize) -> Matrix {
        let Some(document) = file.document(transform) else {
            return root;
        };
        let parent = file_id(document.body.get("m_Father"));
        let above = match parent {
            Some(parent) if depth < 64 => self.world(file, parent, overrides, root, depth + 1),
            _ => root,
        };
        above.then(local(&document.body, transform, overrides))
    }

    fn renderers(&self, file: &File, documents: &[Document], overrides: &Overrides, root: Matrix, nesting: usize, out: &mut Vec<PlacedSprite>, shapes: &mut Vec<(String, Shape)>) {
        for document in documents {
            match document.type_name.as_str() {
                "SpriteRenderer" if !document.stripped => {
                    let Some(owner) = file_id(document.body.get("m_GameObject")) else {
                        continue;
                    };
                    let Some(transform) = self.transform_of(file, owner) else {
                        continue;
                    };
                    let Some(sprite) = document.body.get("m_Sprite") else {
                        continue;
                    };
                    let (Some(guid), Some(sprite_id)) = (sprite.get("guid").and_then(Node::text), file_id(Some(sprite))) else {
                        continue;
                    };
                    let world = self.world(file, transform, overrides, root, 0);
                    let at = self.view.project(world.point([0., 0., 0.]));
                    let flag = |key: &str| document.body.get(key).and_then(Node::text) == Some("1");
                    out.push(PlacedSprite {
                        guid: guid.to_owned(),
                        file_id: sprite_id,
                        at,
                        scale: (world.axis_length(0), world.axis_length(1)),
                        flip: (flag("m_FlipX"), flag("m_FlipY")),
                        order: document.body.get("m_SortingOrder").and_then(Node::number).unwrap_or(0.) as i64,
                        alpha: document.body.at(&["m_Color", "a"]).and_then(Node::number).unwrap_or(1.),
                    });
                }
                "CircleCollider2D" | "BoxCollider2D" | "CapsuleCollider2D" if !document.stripped => {
                    if let Some(shape) = self.shape(file, document, overrides, root) {
                        shapes.push((super::names::component_title(&document.type_name), shape));
                    }
                }
                _ if document.class_id == PREFAB_INSTANCE && nesting < DEEPEST_NESTING => {
                    let parent = file_id(document.body.at(&["m_Modification", "m_TransformParent"]));
                    let above = parent.map_or(root, |parent| self.world(file, parent, overrides, root, 0));
                    self.instance(&document.body, above, nesting + 1, out, shapes);
                }
                _ => {}
            }
        }
    }

    fn transform_of(&self, file: &File, game_object: i64) -> Option<i64> {
        let owner = file.document(game_object)?;
        owner
            .body
            .get("m_Component")
            .map(Node::items)
            .unwrap_or_default()
            .iter()
            .filter_map(|entry| file_id(entry.get("component").or_else(|| entry.entries().first().map(|(_, value)| value))))
            .find(|id| file.document(*id).is_some_and(|component| matches!(component.class_id, TRANSFORM | RECT_TRANSFORM)))
    }

    fn shape(&self, file: &File, collider: &Document, overrides: &Overrides, root: Matrix) -> Option<Shape> {
        let owner = file_id(collider.body.get("m_GameObject"))?;
        let transform = self.transform_of(file, owner)?;
        let world = self.world(file, transform, overrides, root, 0);
        let offset = vector(collider.body.get("m_Offset"), ["x", "y"], [0.; 2]);
        let center = self.view.project(world.point([offset[0], offset[1], 0.]));
        let (sx, sy) = (world.axis_length(0), world.axis_length(1));
        Some(match collider.type_name.as_str() {
            "CircleCollider2D" => Shape::Circle {
                center,
                radius: collider.body.get("m_Radius").and_then(Node::number).unwrap_or(0.5) * sx.max(sy),
            },
            _ => {
                let size = vector(collider.body.get("m_Size"), ["x", "y"], [1.; 2]);
                Shape::Box {
                    center,
                    size: (size[0] * sx, size[1] * sy),
                }
            }
        })
    }

    fn instance(&self, body: &Node, above: Matrix, nesting: usize, out: &mut Vec<PlacedSprite>, shapes: &mut Vec<(String, Shape)>) {
        let Some(guid) = body.at(&["m_SourcePrefab", "guid"]).and_then(Node::text) else {
            return;
        };
        let Some(documents) = self.library.get(guid) else {
            return;
        };
        let file = File::new(documents);
        let overrides = overrides_of(body);
        self.renderers(&file, documents, &overrides, above, nesting, out, shapes);
    }

    fn instance_origin(&self, body: &Node, above: Matrix) -> (f64, f64) {
        let Some(documents) = body.at(&["m_SourcePrefab", "guid"]).and_then(Node::text).and_then(|guid| self.library.get(guid)) else {
            let overrides = overrides_of(body);
            let read = |axis: &str| overrides.iter().find(|((_, path), _)| path == &format!("m_LocalPosition.{axis}")).map_or(0., |(_, value)| *value);
            return self.view.project(above.point([read("x"), read("y"), read("z")]));
        };
        let file = File::new(documents);
        let overrides = overrides_of(body);
        let root = documents
            .iter()
            .find(|document| matches!(document.class_id, TRANSFORM | RECT_TRANSFORM) && file_id(document.body.get("m_Father")).is_none());
        match root {
            Some(root) => self.view.project(self.world(&file, root.file_id, &overrides, above, 0).point([0., 0., 0.])),
            None => self.view.project(above.point([0., 0., 0.])),
        }
    }
}

pub(crate) fn layout(documents: &[Document], model: &Model, library: &HashMap<String, Vec<Document>>, lookup: &Lookup, view: View) -> Layout {
    let placer = Placer { library, lookup, view };
    let file = File::new(documents);
    let none = Overrides::new();
    let mut spots = Vec::new();
    for item in &model.items {
        let mut sprites = Vec::new();
        let mut shapes = Vec::new();
        let mut camera = None;
        let mut marker = Marker::Plain;
        let at = match &item.role {
            Role::Instance { .. } => {
                let Some(document) = file.document(item.key) else {
                    continue;
                };
                let parent = file_id(document.body.at(&["m_Modification", "m_TransformParent"]));
                let above = parent.map_or(Matrix::IDENTITY, |parent| placer.world(&file, parent, &none, Matrix::IDENTITY, 0));
                placer.instance(&document.body, above, 1, &mut sprites, &mut shapes);
                placer.instance_origin(&document.body, above)
            }
            _ => {
                let Some(transform) = placer.transform_of(&file, item.key) else {
                    continue;
                };
                let own: Vec<Document> = file
                    .document(item.key)
                    .map(|owner| {
                        owner
                            .body
                            .get("m_Component")
                            .map(Node::items)
                            .unwrap_or_default()
                            .iter()
                            .filter_map(|entry| file_id(entry.get("component").or_else(|| entry.entries().first().map(|(_, value)| value))))
                            .filter_map(|id| file.document(id).cloned())
                            .collect()
                    })
                    .unwrap_or_default();
                placer.renderers(&file, &own, &none, Matrix::IDENTITY, DEEPEST_NESTING, &mut sprites, &mut shapes);
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
                let world = placer.world(&file, transform, &none, Matrix::IDENTITY, 0);
                view.project(world.point([0., 0., 0.]))
            }
        };
        let _ = &placer.lookup;
        spots.push(Spot {
            key: item.key,
            name: item.name.clone(),
            at,
            sprites,
            shapes,
            camera,
            marker,
        });
    }
    Layout { spots }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Marker, Shape, View, layout, view_for};
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
}
