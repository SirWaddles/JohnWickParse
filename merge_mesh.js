const fs = require('fs');

let mesh1_path = process.argv[2];
let mesh2_path = process.argv[3];

let mesh1 = JSON.parse(fs.readFileSync(mesh1_path));
let mesh2 = JSON.parse(fs.readFileSync(mesh2_path));

let final_mesh = JSON.parse(fs.readFileSync(mesh1_path));

mesh2.buffers.forEach(v => {
    final_mesh.buffers.push(v);
});

mesh2.bufferViews.forEach(v => {
    v.buffer += mesh1.buffers.length;
    final_mesh.bufferViews.push(v);
});

mesh2.accessors.forEach(v => {
    v.bufferView += mesh1.bufferViews.length;
    final_mesh.accessors.push(v);
});

mesh2.images.forEach(v => {
    final_mesh.images.push(v);
});

mesh2.samplers.forEach(v => {
    final_mesh.samplers.push(v);
});

mesh2.textures.forEach(v => {
    v.source += mesh1.images.length;
    v.sampler += mesh1.samplers.length;
    final_mesh.textures.push(v);
});

mesh2.materials.forEach(v => {
    v.pbrMetallicRoughness.baseColorTexture.index += mesh1.textures.length;
    v.normalTexture.index += mesh1.textures.length;
    final_mesh.materials.push(v);
});

let node_names = {};
mesh1.nodes.forEach((v, idx) => {
    if (!v.hasOwnProperty('name')) return;
    node_names[v.name] = idx;
});
let added_nodes = [];
mesh2.nodes.forEach(v => {
    if (!v.hasOwnProperty('name')) return;
    if (node_names.hasOwnProperty(v.name)) return;
    let new_index = final_mesh.nodes.length;
    node_names[v.name] = new_index;
    final_mesh.nodes.push(v);
    added_nodes.push(v);
});

added_nodes.forEach(v => {
    
});

fs.writeFileSync('./merged.gltf', JSON.stringify(final_mesh));
