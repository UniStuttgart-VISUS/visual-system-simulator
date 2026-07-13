$ErrorActionPreference = 'Stop'

$ids = @{
    achromatopsia_blur_factor = 'achromatopsia.blur'
    achromatopsia_int = 'achromatopsia.intensity'
    achromatopsia_onoff = 'achromatopsia.enabled'
    astigmatism_angle_deg = 'eye.astigmatism-angle'
    astigmatism_dpt = 'eye.astigmatism-diopters'
    colorblindness_int = 'retina.color-deficiency-intensity'
    colorblindness_onoff = 'retina.color-deficiency-enabled'
    colorblindness_type = 'retina.color-deficiency-type'
    ct_blur_factor = 'cataract.blur'
    ct_contrast_factor = 'cataract.contrast'
    ct_onoff = 'cataract.enabled'
    eye_axis_rot_x = 'eye.axis-x'
    eye_axis_rot_y = 'eye.axis-y'
    eye_distance_center = 'eye.center-distance'
    glaucoma_fov = 'glaucoma.field'
    glaucoma_onoff = 'glaucoma.enabled'
    maculardegeneration_intadvanced = 'macular.intensity'
    maculardegeneration_inteasy = 'macular.simple-intensity'
    maculardegeneration_onoff = 'macular.enabled'
    maculardegeneration_radius = 'macular.radius'
    maculardegeneration_vadvanced = 'macular.advanced'
    maculardegeneration_veasy = 'macular.simple'
    myopiahyperopia_mnh = 'eye.refraction-diopters'
    myopiahyperopia_onoff = 'eye.refraction-enabled'
    nyctalopia_int = 'nyctalopia.intensity'
    nyctalopia_onoff = 'nyctalopia.enabled'
    peacock_cb_onoff = 'color.enabled'
    peacock_cb_strength = 'color.strength'
    peacock_cb_type = 'color.type'
    presbyopia_near_point = 'eye.near-point'
    presbyopia_onoff = 'eye.presbyopia-enabled'
    receptordensity_onoff = 'retina.receptor-density-enabled'
    retina_map_path = 'retina.map'
    retina_map_pos_x_path = 'retina.map-pos-x'
    retina_map_neg_x_path = 'retina.map-neg-x'
    retina_map_pos_y_path = 'retina.map-pos-y'
    retina_map_neg_y_path = 'retina.map-neg-y'
    retina_map_pos_z_path = 'retina.map-pos-z'
    retina_map_neg_z_path = 'retina.map-neg-z'
}

Get-ChildItem "$PSScriptRoot/../assets/configs" -Filter *.json -Recurse | ForEach-Object {
    $document = Get-Content -LiteralPath $_.FullName -Raw | ConvertFrom-Json -AsHashtable
    $result = [ordered]@{}
    foreach ($eye in @('both', 'left', 'right')) {
        if (-not $document.ContainsKey($eye)) { continue }
        $old = $document[$eye]
        $section = [ordered]@{}
        if ($old.ContainsKey('pose')) {
            foreach ($entry in $old.pose.GetEnumerator()) { $section[$entry.Key] = $entry.Value }
        }
        if ($old.ContainsKey('simulator')) {
            foreach ($entry in $old.simulator.GetEnumerator()) {
                if (-not $ids.ContainsKey($entry.Key)) { throw "No V2 ID for $($entry.Key) in $($_.FullName)" }
                $section[$ids[$entry.Key]] = $entry.Value
            }
        }
        $result[$eye] = $section
    }
    $json = $result | ConvertTo-Json -Depth 10
    Set-Content -LiteralPath $_.FullName -Value $json -Encoding utf8
}
