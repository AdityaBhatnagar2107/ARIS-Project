import * as THREE from 'three';

// 1. Hard Constraints as per A.R.I.S. Spec
const MAX_NODES = 500;
const NODE_GEOMETRY = new THREE.SphereGeometry(0.1, 16, 16);
const NODE_MATERIAL = new THREE.MeshPhongMaterial({ color: 0x00ffcc });

export class GraphRenderer {
  private scene: THREE.Scene;
  private camera: THREE.PerspectiveCamera;
  private renderer: THREE.WebGLRenderer;
  private instancedMesh: THREE.InstancedMesh | null = null;
  private dummy = new THREE.Object3D();

  constructor(canvas: HTMLCanvasElement) {
    this.scene = new THREE.Scene();
    this.camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true });

    const light = new THREE.PointLight(0xffffff, 1, 100);
    light.position.set(10, 10, 10);
    this.scene.add(light, new THREE.AmbientLight(0x404040));
  }

  // 2. Bounded Update (The "No Hairball" Rule)
  public updateGraph(data: { nodes: any[], edges: any[] }) {
    if (data.nodes.length > MAX_NODES) {
      console.warn("Node limit exceeded. Falling back to Micro-view.");
      this.renderFallback(data); // Render a 2D list/tree instead
      return;
    }

    this.cleanup();

    // 3. Instanced Rendering (GPU Efficiency)
    this.instancedMesh = new THREE.InstancedMesh(NODE_GEOMETRY, NODE_MATERIAL, data.nodes.length);

    data.nodes.forEach((node, i) => {
      // Use pre-computed coordinates from Backend
      this.dummy.position.set(node.x || Math.random()*10, node.y || Math.random()*10, node.z || Math.random()*10);
      this.dummy.updateMatrix();
      this.instancedMesh!.setMatrixAt(i, this.dummy.matrix);

      // Store ID for Raycasting/Click events
      this.instancedMesh!.setUserDataAt(i, { id: node.id });
    });

    this.scene.add(this.instancedMesh);
  }

  // 4. Frustum Culling & Performance Loop
  public animate() {
    requestAnimationFrame(() => this.animate());

    // Logic for Semantic Zooming
    const distance = this.camera.position.length();
    this.handleSemanticZoom(distance);

    this.renderer.render(this.scene, this.camera);
  }

  private handleSemanticZoom(distance: number) {
    if (distance > 50) {
      // LEVEL 1: MACRO (Show only directory clusters)
    } else if (distance > 10) {
      // LEVEL 2: MESO (Show file boundaries)
    } else {
      // LEVEL 3: MICRO (Show function-level edges)
    }
  }

  private renderFallback(data: any) {
    // Stub
  }

  private cleanup() {
    if (this.instancedMesh) {
      this.scene.remove(this.instancedMesh);
      this.instancedMesh.geometry.dispose();
      (this.instancedMesh.material as THREE.Material).dispose();
    }
  }
}
