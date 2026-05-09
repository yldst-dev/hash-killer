import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

class AppCheckboxRow extends StatelessWidget {
  const AppCheckboxRow({
    required this.value,
    required this.onChanged,
    required this.child,
    super.key,
  });

  final bool value;
  final ValueChanged<bool>? onChanged;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return FItem.raw(
      onPress: onChanged == null ? null : () => onChanged!(!value),
      prefix: FCheckbox(
        value: value,
        onChange: onChanged,
        enabled: onChanged != null,
      ),
      child: Row(children: [Expanded(child: child)]),
    );
  }
}

class AppRadioRow extends StatelessWidget {
  const AppRadioRow({
    required this.selected,
    required this.onChanged,
    required this.title,
    required this.description,
    super.key,
  });

  final bool selected;
  final VoidCallback onChanged;
  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    return FItem.raw(
      selected: selected,
      onPress: onChanged,
      prefix: FRadio(value: selected, onChange: (_) => onChanged()),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title),
          const SizedBox(height: 3),
          Text(
            description,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: context.theme.typography.sm.copyWith(
              color: context.theme.colors.mutedForeground,
            ),
          ),
        ],
      ),
    );
  }
}
