param(
    [String]
    [Parameter(Mandatory = $true)]
    $pallet,

    [Int]
    $steps = 50,

    [Int]
    $repeat = 20,

    [Switch]
    $w
)

Write-Warning "DO NOT use benchmarking result from Windows"
Write-Warning "This script DOES NOT generate the weight information by default"
Write-Warning "Use -w to do so"

Write-Output "Benchmarking ${pallet} steps ${steps} repeat ${repeat}..."

$cmd = ("./target/release/parami benchmark " +
"--chain=dev " +
"--execution=wasm " +
"--wasm-execution=compiled " +
"--pallet='parami_${pallet}' " +
"--extrinsic='*' " +
"--steps=$steps " +
"--repeat=$repeat")

if ($w) {
    $cmd = "${cmd} " + (
"--template='./.maintain/frame-weight-template.hbs' " +
"--output='./pallets/${pallet}/src/weights.rs'")
}

Invoke-Expression $cmd
