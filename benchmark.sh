set e

declare -a resolutions=("1280 720" "1920 1080" "2560 1440" "3840 2160" "5120 2880" )
for r in $(seq 1 4); do
  for m in $(seq 0 1); do
    for res in "${resolutions[@]}"; do
      ./target/release/vss --config assets/configs/hyperopia-minus2d.json ~/Downloads/classroom4k.rgbd.erp.png --perf 600 --rays $r --mix_type $m --res $res
      cat vss_perf_data.csv >> benchmark_result.csv
    done 
  done
done

declare -a resolutions=("1280 720" "1920 1080" "2560 1440" "3840 2160" )
for r in $(seq 1 4); do
  for m in $(seq 0 1); do
    for res in "${resolutions[@]}"; do
      ./target/release/vss --config_left assets/configs/eye_params_l.json --config_right assets/configs/eye_params_r.json ~/Downloads/classroom4k.rgbd.erp.png --perf 60 --rays $r --mix_type $m --res $res
      cat vss_perf_data.csv >> benchmark_result.csv
    done 
  done
done
