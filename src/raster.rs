pub struct RasterData {
    pub dataset: Dataset,
    pub transform: CoordTransform,
}

impl RasterData {
    pub fn new(path: &str) -> Result<Self, GdalError> {
        let dataset = Dataset::open(path)?;
        let srs = dataset.spatial_ref()?;
        let target_srs = SpatialRef::from_epsg(4326)?;
        let transform = gdal::spatial_ref::CoordTransform::new(&srs, &target_srs)?;
        Ok(Self { dataset, transform })
    }
    // Takes a latitude and longitude in WGS84 coordinates (EPSG:4326) and returns the elevation at that point
    pub fn get_coordinate_height(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<Option<f64>, GdalError> {
        // Copy the input coordinates
        let (lat, lon) = (latitude, longitude);
        
        // Transform the coordinates from everyone's favorite datum (WGS84) to the raster's native coordinate system
        self.transform
            .transform_coords(&mut [lon], &mut [lat], &mut [])?;
        
        // Get the first raster band (usually the only one for elevation data)
        let raster_band = self.dataset.rasterband(1)?;
        
        // Get the affine transformation parameters that map between pixel/line coordinates and georeferenced coordinates
        let transform = self.dataset.geo_transform().unwrap();
        
        // Calculate the pixel (x) and line (y) coordinates in the raster using the affine transform
        // transform[0] = top left x coordinate (origin)
        // transform[1] = pixel width (x resolution)
        // transform[3] = top left y coordinate (origin)
        // transform[5] = pixel height (y resolution, typically negative as y decreases going down)
        let x = (lon - transform[0]) / transform[1];
        let y = (lat - transform[3]) / transform[5];
        
        // Read the elevation value at the calculated pixel position
        // - Reads a 1x1 window at position (x,y)
        // - Uses the Average resampling algorithm (which doesn't matter much for a 1x1 window)
        // - Returns the data as f64 (double precision floating point)
        let mut res_buffer = raster_band.read_as::<f64>(
            (x as isize, y as isize),  // Pixel position (cast to integer)
            (1, 1),                    // Window size to read (1x1 pixel)
            (1, 1),                    // Output buffer size
            Some(ResampleAlg::Average),// Resampling algorithm
        )?;
        
        // Return the elevation value (or None if no data is found)
        // pop() returns and removes the last element from res_buffer.data
        Ok(res_buffer.data.pop())
    }
}

#[test]
fn test_raster_map() {
    let raster_data =
        RasterData::new("assets/Bathymetry/gebco_2023_n47.7905_s39.9243_w25.6311_e42.9895.tif")
            .unwrap();

    // Mt Elbrus
    let tgt_latitude = 43.351851;
    let tgt_longitude = 42.4368771;

    let elevation = raster_data
        .get_coordinate_height(tgt_latitude, tgt_longitude)
        .unwrap()
        .unwrap();

    assert_eq!(elevation, 5392.0);
}