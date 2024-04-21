use bevy::{
    input::common_conditions::{input_just_pressed, input_just_released, input_pressed},
    prelude::*,
    window::PrimaryWindow,
};
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use std::collections::{
    HashMap, HashSet,hash_map
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(CursorWorldPos(None))
        .add_event::<DragEvent>()
        .add_systems(Startup, (setup, generate_elements).chain())
        .add_systems(
            Update,
            (
                get_cursor_world_pos,
                (
                    start_drag.run_if(input_just_pressed(MouseButton::Left)),
                    end_drag.run_if(input_just_pressed(MouseButton::Right)),
                    drag.run_if(input_just_released(MouseButton::Left))
                        .run_if(resource_exists::<DragableElement>),
                    drag_event_listener,
                    draw,
                )
                    .chain(),
            ),
        )
        .run();
}

const ELEMENT_SIZE: f32 = 80.;

#[derive(Copy, Clone)]
enum ElementType {
    Candy = 0,
    Cake = 1,
    Star = 2,
    Lolipop = 3,
    BonBon = 4,
    IceCream = 5,
    Pizza = 6,
    Donut = 7,
}

#[derive(Component, Copy, Clone, PartialEq)]
struct GridPosition {
    x: i32,
    y: i32,
}

#[derive(Event)]
struct DragEvent {
    pos_1: UVec2,
    pos_2: UVec2,
}
#[derive(Component)]
struct Element {
    position: GridPosition,
}

enum SwapError {
    NoGem(UVec2),
    NoMatches,
}
enum MatchDirection {
    Horizontal,
    Vertical,
}

#[derive(Clone)]
pub enum Match {
    /// A straight match of 3 or more gems
    Straight(HashSet<UVec2>),
}

#[derive(Default, Clone)]
pub struct Matches {
    matches: Vec<Match>,
}

impl Matches {
    fn add(&mut self, mat: Match) {
        self.matches.push(mat)
    }

    fn append(&mut self, other: &mut Matches) {
        self.matches.append(&mut other.matches);
    }

    /// Returns the coordinates of all matches in this collection without any repeated values
    pub fn without_duplicates(&self) -> HashSet<UVec2> {
        self.matches
            .iter()
            .flat_map(|mat| match mat {
                Match::Straight(mat) => mat,
            })
            .cloned()
            .collect()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }
}

#[derive(Component, Debug)]
struct Grid {
    width: u32,
    height: u32,
    elements: HashMap<UVec2, u32>,
}


impl std::fmt::Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = (0..self.height).map(|y| {
            f.write_fmt(format_args!(
                "{:?}\n",
                (0..self.width)
                    .map(|x| self.elements[&<[u32; 2] as Into<UVec2>>::into([x, y])])
                    .collect::<Vec<u32>>()
            ))
        });
        for res in res {
            match res {
                Ok(_) => {}
                err => return err,
            }
        }
        Ok(())
    }
}


impl Grid {
    fn get(&self, pos: &UVec2) -> Option<&u32> {
        self.elements.get(pos)
    }

    fn iter(&self) -> hash_map::Iter<UVec2, u32> {
        self.elements.iter()
    }

    fn remove(&mut self, pos: &UVec2) {
        self.elements.remove(pos);
    }

    fn insert(&mut self, pos: UVec2, typ: u32) {
        self.elements.insert(pos, typ);
    }

    fn drop(&mut self) -> HashSet<(UVec2, UVec2)> {
        let mut moves = HashSet::default();
        for x in 0..self.width {
            for y in (0..self.height).rev() {
                if self.get(&[x, y].into()).is_none() {
                    let mut offset = 0;
                    for above in (0..y).rev() {
                        if let Some(typ) = self.get(&[x, above].into()).cloned() {
                            let new_pos = [x, y - offset];
                            moves.insert(([x, above].into(), new_pos.into()));
                            self.remove(&[x, above].into());
                            self.insert(new_pos.into(), typ);
                            offset += 1;
                        }
                    }
                }
            }
        }
        moves
    }

    fn fill(&mut self) -> HashSet<(UVec2, u32)> {
        let mut drops = HashSet::default();
        for x in 0..self.width {
            for y in 0..self.height {
                let pos = [x, y];
                if self.get(&pos.into()).is_none() {
                    let new_type: ElementType = rand::random();
                    self.insert(pos.into(), new_type as u32);
                    drops.insert((pos.into(), new_type as u32));
                }
            }
        }
        drops
    }

    fn swap(&mut self, pos1: &UVec2, pos2: &UVec2) -> Result<(), SwapError> {
        let gem1 = self.get(pos1).copied().ok_or(SwapError::NoGem(*pos1))?;
        let gem2 = self.get(pos2).copied().ok_or(SwapError::NoGem(*pos2))?;
        self.elements.insert(*pos1, gem2);
        self.elements.insert(*pos2, gem1);
        Ok(())
    }

    fn get_matches(&self) -> Matches {
        let mut matches = self.straight_matches(MatchDirection::Horizontal);
        matches.append(&mut self.straight_matches(MatchDirection::Vertical));
        matches
    }

    fn straight_matches(&self, direction: MatchDirection) -> Matches {
        let mut matches = Matches::default();
        let mut current_match = vec![];
        let mut previous_type = None;
        for one in match direction {
            MatchDirection::Horizontal => 0..self.width,
            MatchDirection::Vertical => 0..self.height,
        } {
            for two in match direction {
                MatchDirection::Horizontal => 0..self.height,
                MatchDirection::Vertical => 0..self.width,
            } {
                let pos = [
                    match direction {
                        MatchDirection::Horizontal => one,
                        MatchDirection::Vertical => two,
                    },
                    match direction {
                        MatchDirection::Horizontal => two,
                        MatchDirection::Vertical => one,
                    },
                ]
                .into();

                let current_type = *self.get(&pos).unwrap();
                if current_match.is_empty() || previous_type.unwrap() == current_type {
                    previous_type = Some(current_type);
                    current_match.push(pos);
                } else if previous_type.unwrap() != current_type {
                    match current_match.len() {
                        0..=2 => {}
                        _ => matches.add(Match::Straight(current_match.iter().cloned().collect())),
                    }
                    current_match = vec![pos];
                    previous_type = Some(current_type);
                }
            }
            match current_match.len() {
                0..=2 => {}
                _ => matches.add(Match::Straight(current_match.iter().cloned().collect())),
            }
            current_match = vec![];
            previous_type = None;
        }
        matches
    }

    fn clear_matches(&mut self) {
        loop {
            let matches = self.get_matches();
            if matches.is_empty() {
                break;
            }
            for mat in matches.matches.iter() {
                match mat {
                    Match::Straight(elements) => {
                        for element in elements {
                            self.remove(element);
                        }
                    }
                }
            }
            self.drop();
            self.fill();
        }
    }

}

#[derive(Resource)]
struct CursorWorldPos(Option<Vec2>);

#[derive(Resource)]
struct DragableElement(GridPosition);

fn setup(
    mut commands: Commands,
    q_window: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
) {
    let window = q_window.single();
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(window.width() / 2., window.height() / 2., 0.),
        ..default()
    });
    let grid = Grid {
        width: 8,
        height: 8,
        elements: HashMap::new(),
    };
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1., 1., 1.).into(),
                rect: Some(Rect::new(
                    0.,
                    0.,
                    ELEMENT_SIZE * grid.width as f32,
                    ELEMENT_SIZE * grid.height as f32,
                )),
                ..default()
            },
            transform: Transform::from_xyz(window.width() / 2., window.height() / 2., 0.),
            ..default()
        },
        grid,
    ));
}

fn generate_elements(
    mut commands: Commands,
    mut q_grid: Query<&mut Grid>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
) {
    let window = q_window.single();
    if let Ok(mut grid) = q_grid.get_single_mut() {
        for _y in 0..grid.height {
            for _x in 0..grid.width {
                let element_type: ElementType = rand::random();
                grid.elements
                    .insert(UVec2 { x: _x, y: _y }, element_type as u32);
            }
        }
    }
}

fn draw(
    mut commands: Commands,
    mut q_grid: Query<&mut Grid>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_elements: Query<(&Element, Entity)>,
    asset_server: Res<AssetServer>,
) {
    let window = q_window.single();
    if let Ok(mut grid) = q_grid.get_single_mut() {
        let left_up_corner = Vec2 {
            x: window.width() / 2. - (grid.width as f32 / 2. * ELEMENT_SIZE),
            y: window.height() / 2. - (grid.height as f32 / 2. * ELEMENT_SIZE),
        };

        for (_, entity)in q_elements.iter(){
            commands.entity(entity).despawn();  
        }
        for _y in 0..grid.height {
            for _x in 0..grid.width {
                if !grid.get(&[_x, _y].into()).is_none(){
                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                custom_size: Some(Vec2 {
                                    x: ELEMENT_SIZE,
                                    y: ELEMENT_SIZE,
                                }),
                                ..default()
                            },
                            texture: asset_server.load(format!(
                                "sprites/{}.png",
                                grid.elements[&UVec2 { x: _x, y: _y }]
                            )),
                            transform: Transform::from_xyz(
                                (left_up_corner.x + ELEMENT_SIZE / 2.) + _x as f32 * ELEMENT_SIZE,
                                (left_up_corner.y + ELEMENT_SIZE / 2.) + _y as f32 * ELEMENT_SIZE,
                                1.,
                            ),
                            ..default()
                        },
                        Element {
                            position: GridPosition {
                                x: _x as i32,
                                y: _y as i32,
                            },
                        },
                    ));
                } 
            }
        }
    }
}

// Получаем координаты курсора и сохраняем как ресурс
fn get_cursor_world_pos(
    mut cursor_world_pos: ResMut<CursorWorldPos>,
    q_primary_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
) {
    let primary_window = q_primary_window.single();
    let (main_camera, main_camera_transform) = q_camera.single();
    cursor_world_pos.0 = primary_window
        .cursor_position()
        .and_then(|cursor_pos| main_camera.viewport_to_world_2d(main_camera_transform, cursor_pos));
}

fn start_drag(
    mut commands: Commands,
    cursor_world_pos: Res<CursorWorldPos>,
    q_elements: Query<(&Transform, &Element, Entity)>,
) {
    // If the cursor is not within the primary window skip this system
    let Some(cursor_world_pos) = cursor_world_pos.0 else {
        return;
    };
    info!("left mouse just pressed");
    for (transform, element, entity) in q_elements.iter() {
        let distance = cursor_world_pos - transform.translation.truncate();
        if (distance.length() < ELEMENT_SIZE / 2.) {
            commands.insert_resource(DragableElement(element.position));
        }
    }
}

fn end_drag(mut commands: Commands) {
    commands.remove_resource::<DragableElement>();
}

fn drag(
    mut draggable_element: Res<DragableElement>,
    cursor_world_pos: Res<CursorWorldPos>,
    mut drag_events: EventWriter<DragEvent>,
    mut q_mut_elements: Query<(&mut Transform, &mut Element)>,
) {
    // If the cursor is not within the primary window skip this system
    let Some(cursor_world_pos) = cursor_world_pos.0 else {
        return;
    };
    info!("left mouse just released and resource exist");
    for (transform, mut element) in q_mut_elements.iter_mut() {
        let distance = cursor_world_pos - transform.translation.truncate();
        if (distance.length() < ELEMENT_SIZE / 2.) {
            info!("Send first event");
            drag_events.send(DragEvent {
                pos_1: UVec2 { x: draggable_element.0.x as u32, y: draggable_element.0.y as u32},
                pos_2: UVec2 { x: element.position.x as u32, y: element.position.y as u32},
            });
            break;
        }
    }
}

fn drag_event_listener(
    mut events: EventReader<DragEvent>,
    mut q_mut_elements: Query<(&mut Element, Entity)>,
    mut q_grid: Query<&mut Grid>,
) {
    for drag_event in events.read() {
        info!("Event handle");
        if let Ok(mut grid) = q_grid.get_single_mut() {
            grid.swap(&drag_event.pos_1, &drag_event.pos_2);
            println!("{:?}",grid);
            let matches = grid.get_matches();
            println!("{}", matches.matches.len());
            grid.clear_matches();
            let matches = grid.get_matches();
            println!("{}", matches.matches.len());
        }
    }
}

impl Distribution<ElementType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ElementType {
        match rng.gen_range(0..=7) {
            0 => ElementType::Candy,
            1 => ElementType::Cake,
            2 => ElementType::Star,
            3 => ElementType::Lolipop,
            4 => ElementType::BonBon,
            5 => ElementType::IceCream,
            6 => ElementType::Pizza ,
            _ => ElementType::Donut ,
        }
    }
}
