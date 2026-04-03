import sys, json

data = json.loads(sys.stdin.read())
types = data.get('data', {})
print('\n{:<30} {:<7} {:<10} {:<8} {}'.format('Instance Type','vCPUs','RAM(GiB)','USD/hr','Available Regions'))
print('-' * 100)
for name, info in sorted(types.items()):
    specs = info.get('instance_type', {})
    price = specs.get('price_cents_per_hour', 0) / 100
    vcpus = specs.get('vcpus', '?')
    ram   = specs.get('memory_gib', '?')
    avail = info.get('regions_with_capacity_available', [])
    regions = ', '.join([r.get('name','?') for r in avail]) or '-- none --'
    print('{:<30} {:<7} {:<10} ${:<7.2f} {}'.format(name, str(vcpus), str(ram), price, regions))
print()
