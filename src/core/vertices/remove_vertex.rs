use rayon::prelude::*;

use crate::{
    HyperedgeKey,
    HyperedgeTrait,
    Hypergraph,
    VertexIndex,
    VertexTrait,
    errors::HypergraphError,
};

impl<V, HE> Hypergraph<V, HE>
where
    V: VertexTrait,
    HE: HyperedgeTrait,
{
    /// Removes a vertex by index.
    pub fn remove_vertex(
        &mut self,
        vertex_index: VertexIndex,
    ) -> Result<(), HypergraphError<V, HE>> {
        let internal_index = self.get_internal_vertex(vertex_index)?;

        // Get the hyperedges of the vertex.
        let hyperedges =
            self.get_internal_hyperedges(&self.get_vertex_hyperedges(vertex_index)?)?;

        // Remove the vertex from the hyperedges which contain it.
        for hyperedge in hyperedges {
            let HyperedgeKey { vertices, .. } = self
                .hyperedges
                .get_index(hyperedge)
                .cloned()
                .ok_or(HypergraphError::InternalHyperedgeIndexNotFound(hyperedge))?;

            let hyperedge_index = self.get_hyperedge(hyperedge)?;

            // Get the unique vertices, i.e. check for self-loops.
            let mut unique_vertices = vertices.clone();

            // We use `par_sort_unstable` here which means that the order of
            // equal elements is not preserved but this is fine since we dedupe
            // them afterwards.
            unique_vertices.par_sort_unstable();
            unique_vertices.dedup();

            // Remove the hyperedge if the vertex is the only one present.
            if unique_vertices.len() == 1 {
                self.remove_hyperedge(hyperedge_index)?;
            } else {
                // Otherwise update the hyperedge with the updated vertices.
                let updated_vertices = self.get_vertices(
                    &vertices
                        .into_par_iter()
                        .filter(|vertex| *vertex != internal_index)
                        .collect::<Vec<usize>>(),
                )?;

                self.update_hyperedge_vertices(hyperedge_index, updated_vertices)?;
            }
        }

        // Find the last index.
        let last_index = self.vertices.len() - 1;

        // Swap and remove by index.
        self.vertices.swap_remove_index(internal_index);

        // Update the mapping for the removed vertex.
        self.vertices_mapping.left.remove(&internal_index);
        self.vertices_mapping.right.remove(&vertex_index);

        // If the index to remove wasn't the last one, the last vertex has
        // been swapped in place of the removed one. See the remove_hyperedge
        // method for more details about the internals.
        if internal_index != last_index {
            // Get the index of the swapped vertex.
            let swapped_vertex_index = self.get_vertex(last_index)?;

            // Proceed with the aforementioned operations.
            self.vertices_mapping
                .right
                .insert(swapped_vertex_index, internal_index);
            self.vertices_mapping.left.remove(&last_index);
            self.vertices_mapping
                .left
                .insert(internal_index, swapped_vertex_index);

            let stale_hyperedges =
                self.get_internal_hyperedges(&self.get_vertex_hyperedges(swapped_vertex_index)?)?;

            // Update the impacted hyperedges accordingly.
            for hyperedge in stale_hyperedges {
                let HyperedgeKey { vertices, weight } = self
                    .hyperedges
                    .get_index(hyperedge)
                    .ok_or(HypergraphError::InternalHyperedgeIndexNotFound(hyperedge))?;

                let updated_vertices = vertices
                    .into_par_iter()
                    .map(|vertex| {
                        // Remap the vertex if this is the swapped one.
                        if vertex == &last_index {
                            internal_index
                        } else {
                            *vertex
                        }
                    })
                    .collect();

                // Insert the new entry with the updated vertices.
                // Since we are not altering the weight, we can safely perform
                // the operation without checking its output.
                self.hyperedges
                    .insert(HyperedgeKey::new(updated_vertices, *weight));

                // Swap and remove by index.
                // Since we know that the hyperedge index is correct, we can
                // safely perform the operation without checking its output.
                self.hyperedges.swap_remove_index(hyperedge);
            }
        }

        // Return a unit.
        Ok(())
    }
}
