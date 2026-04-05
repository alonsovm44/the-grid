use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridFile {
    pub id: Uuid,
    pub name: String,
    pub content: Vec<u8>,
    pub tags: HashSet<String>,
    pub position: [f32; 3], // XYZ coordinates in virtual space
    pub velocity: [f32; 3], // Movement in space (for dynamic organization)
    pub mass: f32,       // Resistance to movement (file importance)
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub access_frequency: f64,  // How often accessed (affects position)
    pub relationships: HashMap<Uuid, RelationshipType>, // Related files and relationship type
    pub semantic_vector: Option<Vec<f32>>, // AI-generated semantic embedding
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    DependsOn,      // This file needs another to function
    Implements,     // This file implements concepts from another
    Tests,          // This file tests another
    Documents,      // This file documents another
    ConflictsWith,  // This file conflicts with another
    Enhances,       // This file enhances another
    Replaces,       // This file replaces another
}

#[derive(Debug, Clone)]
pub struct SpatialPhysics {
    pub attraction_coefficient: f32,
    pub repulsion_radius: f32,
    pub semantic_gravity: f32,
    pub usage_heat_coefficient: f32,
    pub temporal_drift: f32,
}

impl Default for SpatialPhysics {
    fn default() -> Self {
        Self {
            attraction_coefficient: 0.1,
            repulsion_radius: 50.0,
            semantic_gravity: 0.05,
            usage_heat_coefficient: 0.2,
            temporal_drift: 0.01,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TagCluster {
    pub tag: String,
    pub center: [f32; 3],
    pub radius: f32,
    pub density: f32,
    pub files: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct SpatialKnowledgeFS {
    pub files: HashMap<Uuid, GridFile>,
    pub spatial_index: SpatialIndex3D,
    pub tag_index: TagIndex,
    pub physics: SpatialPhysics,
    pub tag_clusters: HashMap<String, TagCluster>,
}

#[derive(Debug, Clone)]
pub struct SpatialIndex3D {
    // Octree or spatial grid for fast 3D queries
    pub grid_size: f32,
    pub cell_size: f32,
}

#[derive(Debug, Clone)]
pub struct TagIndex {
    pub tag_to_files: HashMap<String, Vec<Uuid>>,
    pub file_to_tags: HashMap<Uuid, Vec<String>>,
}

impl SpatialKnowledgeFS {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            spatial_index: SpatialIndex3D {
                grid_size: 1000.0,
                cell_size: 50.0,
            },
            tag_index: TagIndex {
                tag_to_files: HashMap::new(),
                file_to_tags: HashMap::new(),
            },
            physics: SpatialPhysics::default(),
            tag_clusters: HashMap::new(),
        }
    }

    pub fn add_file(&mut self, file: GridFile) {
        let id = file.id;
        
        // Add to main storage
        self.files.insert(id, file.clone());
        
        // Update tag indexes
        for tag in &file.tags {
            self.tag_index.tag_to_files
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(id);
            
            self.tag_index.file_to_tags
                .entry(id)
                .or_insert_with(Vec::new)
                .push(tag.clone());
        }
        
        // Update spatial index
        self.update_spatial_organization();
    }

    pub fn get_file(&self, id: &Uuid) -> Option<&GridFile> {
        self.files.get(id)
    }

    pub fn get_file_mut(&mut self, id: &Uuid) -> Option<&mut GridFile> {
        self.files.get_mut(id)
    }

    pub fn find_files_by_tag(&self, tag: &str) -> Vec<&GridFile> {
        self.tag_index.tag_to_files
            .get(tag)
            .map(|file_ids| {
                file_ids.iter()
                    .filter_map(|id| self.files.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn find_files_within_radius(&self, position: [f32; 3], radius: f32) -> Vec<&GridFile> {
        self.files.values()
            .filter(|file| {
                let distance = ((file.position[0] - position[0]).powi(2) +
                               (file.position[1] - position[1]).powi(2) +
                               (file.position[2] - position[2]).powi(2)).sqrt();
                distance <= radius
            })
            .collect()
    }

    pub fn find_semantically_similar_files(&self, file_id: &Uuid, threshold: f32) -> Vec<&GridFile> {
        if let Some(file) = self.files.get(file_id) {
            if let Some(file_vector) = &file.semantic_vector {
                self.files.values()
                    .filter(|other| {
                        if let Some(other_vector) = &other.semantic_vector {
                            file_vector.iter().zip(other_vector.iter())
                                .map(|(a, b)| a * b)
                                .sum::<f32>() / (file_vector.len() as f32) > threshold
                        } else {
                            false
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    pub fn add_relationship(&mut self, from_id: Uuid, to_id: Uuid, relationship: RelationshipType) {
        if let Some(from_file) = self.files.get_mut(&from_id) {
            from_file.relationships.insert(to_id, relationship);
        }
    }

    pub fn get_related_files(&self, file_id: &Uuid) -> Vec<(&Uuid, &RelationshipType, &GridFile)> {
        if let Some(file) = self.files.get(file_id) {
            file.relationships
                .iter()
                .filter_map(|(related_id, rel_type)| {
                    self.files.get(related_id)
                        .map(|related_file| (related_id, rel_type, related_file))
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    fn update_spatial_organization(&mut self) {
        // Simple spatial clustering based on tags and relationships
        self.create_tag_clusters();
        
        // Apply simple physics to update positions
        self.apply_spatial_forces();
    }

    fn create_tag_clusters(&mut self) {
        self.tag_clusters.clear();
        
        // Group files by tags
        let mut tag_groups: HashMap<String, Vec<Uuid>> = HashMap::new();
        
        for (id, file) in &self.files {
            for tag in &file.tags {
                tag_groups.entry(tag.clone()).or_insert_with(Vec::new).push(*id);
            }
        }
        
        // Create clusters for each tag
        for (tag, file_ids) in tag_groups {
            if file_ids.len() > 1 {
                let center = self.calculate_cluster_center(&file_ids);
                let radius = self.calculate_cluster_radius(&file_ids);
                
                let cluster = TagCluster {
                    tag: tag.clone(),
                    center,
                    radius,
                    density: file_ids.len() as f32,
                    files: file_ids,
                };
                
                self.tag_clusters.insert(tag, cluster);
            }
        }
    }

    fn calculate_cluster_center(&self, file_ids: &[Uuid]) -> [f32; 3] {
        let mut center = [0.0, 0.0, 0.0];
        
        for id in file_ids {
            if let Some(file) = self.files.get(id) {
                center[0] += file.position[0];
                center[1] += file.position[1];
                center[2] += file.position[2];
            }
        }
        
        if !file_ids.is_empty() {
            let count = file_ids.len() as f32;
            center[0] /= count;
            center[1] /= count;
            center[2] /= count;
        }
        
        center
    }

    fn calculate_cluster_radius(&self, file_ids: &[Uuid]) -> f32 {
        if file_ids.is_empty() {
            return 0.0;
        }
        
        let center = self.calculate_cluster_center(file_ids);
        let mut max_distance: f32 = 0.0;
        
        for id in file_ids {
            if let Some(file) = self.files.get(id) {
                let distance = ((file.position[0] - center[0]).powi(2) +
                               (file.position[1] - center[1]).powi(2) +
                               (file.position[2] - center[2]).powi(2)).sqrt();
                max_distance = max_distance.max(distance);
            }
        }
        
        max_distance + 10.0 // Add some padding
    }

    fn apply_spatial_forces(&mut self) {
        // This is a simplified version - in a full implementation,
        // we'd apply attraction/repulsion forces between related files
        
        for file in self.files.values_mut() {
            // Simple drift toward cluster centers for files with shared tags
            for tag in &file.tags {
                if let Some(cluster) = self.tag_clusters.get(tag) {
                    let direction = [
                        cluster.center[0] - file.position[0],
                        cluster.center[1] - file.position[1],
                        cluster.center[2] - file.position[2],
                    ];
                    
                    let distance = direction.iter().map(|d| d * d).sum::<f32>().sqrt();
                    if distance > 1.0 {
                        let force = 0.01 * self.physics.temporal_drift;
                        file.velocity[0] += direction[0] * force;
                        file.velocity[1] += direction[1] * force;
                        file.velocity[2] += direction[2] * force;
                    }
                }
            }
            
            // Apply velocity with damping
            file.position[0] += file.velocity[0];
            file.position[1] += file.velocity[1];
            file.position[2] += file.velocity[2];
            
            // Damping
            file.velocity[0] *= 0.95;
            file.velocity[1] *= 0.95;
            file.velocity[2] *= 0.95;
        }
    }

    pub fn increment_access_frequency(&mut self, file_id: &Uuid) {
        if let Some(file) = self.files.get_mut(file_id) {
            file.access_frequency += 1.0;
            
            // Update spatial organization when access patterns change
            self.update_spatial_organization();
        }
    }

    pub fn create_file_with_position(&mut self, name: String, content: Vec<u8>, tags: HashSet<String>, position: [f32; 3]) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        let file = GridFile {
            id,
            name: name.clone(),
            content,
            tags,
            position,
            velocity: [0.0, 0.0, 0.0],
            mass: 1.0,
            created_at: now,
            modified_at: now,
            access_frequency: 0.0,
            relationships: HashMap::new(),
            semantic_vector: None,
        };
        
        self.add_file(file);
        id
    }

    pub fn scan_directory(&mut self, path: &str) {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        
                        // Skip files already in SKFS to avoid duplicates
                        if self.files.values().any(|f| f.name == name) {
                            continue;
                        }

                        // Deterministic position based on name hashing (Milestone 1.3)
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        name.hash(&mut hasher);
                        let hash = hasher.finish();
                        
                        let x = (hash % 400) as f32 - 200.0;
                        let y = ((hash >> 8) % 400) as f32 - 200.0;
                        let z = ((hash >> 16) % 400) as f32 - 200.0;
                        
                        let mut tags = HashSet::new();
                        if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                            match ext {
                                "rs" => { tags.insert("source".to_string()); tags.insert("rust".to_string()); },
                                "md" => { tags.insert("docs".to_string()); tags.insert("markdown".to_string()); },
                                "toml" => { tags.insert("config".to_string()); tags.insert("toml".to_string()); },
                                _ => { tags.insert("data".to_string()); }
                            }
                        }

                        self.create_file_with_position(name, Vec::new(), tags, [x, y, z]);
                    }
                }
            }
        }
    }

    pub fn initialize_with_defaults(&mut self) {
        // Create some default directories/files in alonso/ (user space)
        let mut default_files = Vec::new();
        
        // Create basic project structure as starting point
        let readme_tags = HashSet::from(["documentation".to_string(), "welcome".to_string()]);
        let readme_content = b"# Welcome to The Grid\n\nThis is your spatial knowledge base. Files here are organized by meaning, not hierarchy.";
        let readme_id = self.create_file_with_position(
            "README.md".to_string(),
            readme_content.to_vec(),
            readme_tags,
            [0.0, 0.0, 0.0],
        );
        default_files.push(readme_id);
        
        let config_tags = HashSet::from(["configuration".to_string(), "system".to_string()]);
        let config_content = b"# Grid Configuration\n\nThis file stores your Grid preferences and settings.";
        let config_id = self.create_file_with_position(
            "grid_config.toml".to_string(),
            config_content.to_vec(),
            config_tags,
            [50.0, 0.0, 0.0],
        );
        default_files.push(config_id);
        
        // Create some example source files
        for i in 1..=3 {
            let source_tags = HashSet::from(["source".to_string(), "example".to_string(), format!("module_{}", i)]);
            let source_content = format!("// Example module {}\nfn example_{}() {{\n    println!(\"Hello from module {}\");\n}}\n", i, i, i).into_bytes();
            let source_id = self.create_file_with_position(
                format!("example_{}.rs", i),
                source_content,
                source_tags,
                [i as f32 * 30.0, 0.0, 0.0],
            );
            default_files.push(source_id);
        }
        
        // Update spatial organization after creating defaults
        self.update_spatial_organization();
    }
}
