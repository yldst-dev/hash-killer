import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

class AppTextField extends StatelessWidget {
  const AppTextField({
    required this.value,
    required this.onChanged,
    this.label,
    this.hint,
    this.keyboardType,
    this.minLines,
    this.maxLines = 1,
    super.key,
  });

  final String value;
  final ValueChanged<String> onChanged;
  final String? label;
  final String? hint;
  final TextInputType? keyboardType;
  final int? minLines;
  final int? maxLines;

  @override
  Widget build(BuildContext context) {
    return FTextField(
      control: FTextFieldControl.lifted(
        value: TextEditingValue(
          text: value,
          selection: TextSelection.collapsed(offset: value.length),
        ),
        onChange: (value) => onChanged(value.text),
      ),
      label: label == null ? null : Text(label!),
      hint: hint,
      keyboardType: keyboardType,
      minLines: minLines,
      maxLines: maxLines,
    );
  }
}
